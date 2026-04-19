import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { accountService, statusService } from '@/services'
import type { Account, CreateAccountInput, LocalAuthSyncResult, UpdateAccountInput } from '@/types'

export const useAccountStore = defineStore('account', () => {
  // 状态
  const accounts = ref<Account[]>([])
  const activeAccountId = ref<string | null>(null)
  const loading = ref(false)
  const checkingStatus = ref<Set<string>>(new Set())

  // 派生数据
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

  // 操作
  async function loadAccounts() {
    loading.value = true
    try {
      const defaultSyncResult = await accountService.syncLocalDefaultAccount()
      accounts.value = await accountService.listAccounts()
      const syncedDefaultId = defaultSyncResult.matched_account_id ?? null
      const syncedDefaultExists = accounts.value.some((account) => account.id === syncedDefaultId)
      const activeAccountExists = accounts.value.some((account) => account.id === activeAccountId.value)

      if (syncedDefaultId && syncedDefaultExists) {
        activeAccountId.value = syncedDefaultId
      } else if (!activeAccountId.value || !activeAccountExists) {
        activeAccountId.value = defaultAccount.value?.id ?? accounts.value[0]?.id ?? null
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
    if (!accounts.value.some((account) => account.id === id)) return
    await accountService.switchAccount(id)
    activeAccountId.value = id
    await loadAccounts()
  }

  async function setActive(id: string) {
    activeAccountId.value = id
  }

  async function syncLocalAuthFile(): Promise<LocalAuthSyncResult> {
    const result = await accountService.syncLocalAuthFile()
    await loadAccounts()
    if (accounts.value.some((account) => account.id === result.account_id)) {
      activeAccountId.value = result.account_id
    }
    return result
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
      await loadAccounts()
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
      await loadAccounts()
    } finally {
      checkingStatus.value.clear()
    }
  }

  function upsertAccountSnapshot(snapshot: Account) {
    const existingIndex = accounts.value.findIndex((account) => account.id === snapshot.id)
    if (existingIndex >= 0) {
      accounts.value[existingIndex] = snapshot
    } else {
      accounts.value.push(snapshot)
    }

    if (snapshot.is_default) {
      activeAccountId.value = snapshot.id
    }
  }

  function updateAccountStatusFromEvent(
    accountId: string,
    status: string,
    message?: string,
    accountSnapshot?: Account,
  ) {
    if (accountSnapshot) {
      upsertAccountSnapshot(accountSnapshot)
      return
    }

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
    syncLocalAuthFile,
    checkAccountStatus,
    checkAllStatus,
    upsertAccountSnapshot,
    updateAccountStatusFromEvent,
  }
})
