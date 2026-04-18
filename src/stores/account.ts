import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { accountService, statusService } from '@/services'
import type { Account, CreateAccountInput, UpdateAccountInput } from '@/types'

export const useAccountStore = defineStore('account', () => {
  // State
  const accounts = ref<Account[]>([])
  const activeAccountId = ref<string | null>(null)
  const loading = ref(false)
  const checkingStatus = ref<Set<string>>(new Set())

  // Getters
  const defaultAccount = computed(() =>
    accounts.value.find((a) => a.is_default) ?? accounts.value[0] ?? null
  )

  const activeAccount = computed(() =>
    accounts.value.find((a) => a.id === activeAccountId.value) ?? defaultAccount.value
  )

  const totalAccounts = computed(() => accounts.value.length)

  const accountsByStatus = computed(() => {
    const map: Record<string, Account[]> = {
      normal: [],
      warning: [],
      error: [],
      expired: [],
      unknown: [],
    }
    for (const acc of accounts.value) {
      map[acc.status]?.push(acc)
    }
    return map
  })

  // Actions
  async function loadAccounts() {
    loading.value = true
    try {
      accounts.value = await accountService.listAccounts()
      if (!activeAccountId.value && accounts.value.length > 0) {
        activeAccountId.value = defaultAccount.value?.id ?? accounts.value[0].id
      }
    } finally {
      loading.value = false
    }
  }

  async function createAccount(input: CreateAccountInput): Promise<Account> {
    const account = await accountService.createAccount(input)
    accounts.value.push(account)
    if (accounts.value.length === 1) {
      activeAccountId.value = account.id
    }
    return account
  }

  async function updateAccount(input: UpdateAccountInput): Promise<Account> {
    const updated = await accountService.updateAccount(input)
    const idx = accounts.value.findIndex((a) => a.id === updated.id)
    if (idx !== -1) accounts.value[idx] = updated
    return updated
  }

  async function deleteAccount(id: string) {
    await accountService.deleteAccount(id)
    accounts.value = accounts.value.filter((a) => a.id !== id)
    if (activeAccountId.value === id) {
      activeAccountId.value = accounts.value[0]?.id ?? null
    }
  }

  async function switchAccount(id: string) {
    await accountService.switchAccount(id)
    activeAccountId.value = id
    // Refresh list to get updated is_default flags
    await loadAccounts()
  }

  async function setActive(id: string) {
    activeAccountId.value = id
  }

  async function checkAccountStatus(id: string) {
    checkingStatus.value.add(id)
    try {
      const result = await statusService.checkStatus(id)
      const acc = accounts.value.find((a) => a.id === id)
      if (acc) {
        acc.status = result.status
        acc.status_message = result.message
      }
    } finally {
      checkingStatus.value.delete(id)
    }
  }

  async function checkAllStatus() {
    accounts.value.forEach((a) => checkingStatus.value.add(a.id))
    try {
      const results = await statusService.checkAllStatus()
      for (const result of results) {
        const acc = accounts.value.find((a) => a.id === result.account_id)
        if (acc) {
          acc.status = result.status
          acc.status_message = result.message
        }
      }
    } finally {
      checkingStatus.value.clear()
    }
  }

  function updateAccountStatusFromEvent(accountId: string, status: string, message?: string) {
    const acc = accounts.value.find((a) => a.id === accountId)
    if (acc) {
      acc.status = status as Account['status']
      acc.status_message = message
    }
  }

  return {
    accounts,
    activeAccountId,
    activeAccount,
    defaultAccount,
    loading,
    checkingStatus,
    totalAccounts,
    accountsByStatus,
    loadAccounts,
    createAccount,
    updateAccount,
    deleteAccount,
    switchAccount,
    setActive,
    checkAccountStatus,
    checkAllStatus,
    updateAccountStatusFromEvent,
  }
})
