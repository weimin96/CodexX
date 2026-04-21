<template>
  <div class="app-page">
    <section v-if="loading" class="surface-panel empty-panel">
      <n-spin size="medium" />
      <p>正在加载账号。</p>
    </section>

    <section v-else class="surface-panel section-grid account-section">
      <div class="account-grid-header">
        <div>
          <h2 class="panel-heading">账号信息</h2>
        </div>
        <div class="account-header-actions">
          <div class="search-row">
            <n-input
              v-model:value="searchQuery"
              placeholder="搜索邮箱或账号标识..."
              clearable
            >
              <template #prefix>
                <n-icon><SearchIcon /></n-icon>
              </template>
            </n-input>
          </div>
          <n-dropdown
            trigger="click"
            :options="accountActionOptions"
            @select="handleAccountActionSelect"
          >
            <n-button
              secondary
              class="account-action-trigger"
              title="操作"
              aria-label="操作"
            >
              <span class="account-action-content">
                <span>操作</span>
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <path
                    d="M12 5h.01M12 12h.01M12 19h.01"
                    stroke="currentColor"
                    stroke-width="2.4"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </span>
            </n-button>
          </n-dropdown>
        </div>
      </div>

      <section v-if="warmupProgressVisible" class="warmup-progress-panel">
        <div class="warmup-progress-header">
          <div class="warmup-progress-copy">
            <h3>{{ warmupProgressTitle }}</h3>
            <p>{{ warmupProgressSummary }}</p>
          </div>
          <n-button
            v-if="!triggeringConversation"
            quaternary
            size="small"
            @click="clearWarmupProgress"
          >
            清空
          </n-button>
        </div>

        <n-progress type="line" :percentage="warmupProgressPercentage" />

        <div class="warmup-progress-meta">
          <span class="warmup-progress-chip">可执行 {{ warmupExecutableCount }}</span>
          <span class="warmup-progress-chip">成功 {{ warmupSuccessCount }}</span>
          <span class="warmup-progress-chip">失败 {{ warmupFailedCount }}</span>
        </div>

        <div class="warmup-progress-list">
          <article
            v-for="entry in warmupProgressEntries"
            :key="entry.accountId"
            class="warmup-progress-item"
            :class="`state-${entry.state}`"
          >
            <div class="warmup-progress-item-copy">
              <strong>{{ entry.accountName }}</strong>
              <span>{{ entry.detail }}</span>
            </div>
            <span class="warmup-progress-item-status">
              {{ resolveWarmupStateLabel(entry.state) }}
            </span>
          </article>
        </div>
      </section>

      <div v-if="!hasAccounts" class="inline-empty-state">
        <p>还没有添加任何账号，请通过操作添加、导入或同步账号。</p>
      </div>

      <div v-else-if="filteredAccounts.length === 0" class="inline-empty-state">
        <p>没有匹配的账号。</p>
      </div>

      <div v-else class="account-grid">
        <AccountCard
          v-for="account in filteredAccounts"
          :key="account.id"
          :account="account"
          :checking="checkingStatus.has(account.id)"
          :refreshing-token="refreshingTokenAccounts.has(account.id)"
          :triggering-conversation="triggeringConversationAccounts.has(account.id)"
          :warmup-disabled="triggeringConversation"
          @detail="navigateToDetail(account.id)"
          @check="handleCheckStatus(account.id)"
          @refresh-token="handleRefreshToken(account.id)"
          @switch-account="handleSwitchAccount(account.id)"
          @export-auth="handleExportAccount(account)"
          @trigger-conversation="handleTriggerConversation(account.id)"
          @delete="handleDelete(account)"
        />
      </div>
    </section>

    <CreateAccountModal v-model:show="showCreateModal" @created="handleCreated" />

    <n-modal
      :show="showOAuthModal"
      preset="card"
      title="OAuth 网页登录"
      class="oauth-login-modal"
      style="width: min(700px, calc(100vw - 24px));"
      @update:show="handleOAuthModalVisibleChange"
    >
      <div class="oauth-modal-layout">
        <section class="oauth-hero-card">
          <div class="oauth-hero-copy">
            <span class="oauth-eyebrow">OpenAI 授权</span>
            <h3>通过系统浏览器完成 OAuth 登录</h3>
            <p>应用只监听 127.0.0.1 本地回调，并将返回结果加密保存到本地账号库。</p>
          </div>
          <div class="oauth-step-grid">
            <article class="oauth-step-card">
              <span class="oauth-step-index">01</span>
              <strong>打开授权页</strong>
              <p>自动拉起浏览器进入 OpenAI 授权流程。</p>
            </article>
            <article class="oauth-step-card">
              <span class="oauth-step-index">02</span>
              <strong>等待本地回调</strong>
              <p>默认监听 127.0.0.1 回调地址接收登录结果。</p>
            </article>
            <article class="oauth-step-card">
              <span class="oauth-step-index">03</span>
              <strong>必要时手动补录</strong>
              <p>浏览器未回跳时，可手动粘贴回调链接完成登录。</p>
            </article>
          </div>
        </section>

        <section class="oauth-panel">
          <div class="oauth-panel-head">
            <div>
              <h4>浏览器授权</h4>
              <p>当前会话已生成专用授权链接和本地回调地址。</p>
            </div>
            <span class="oauth-status-pill" :class="{ active: oauthWaitingForCallback }">
              {{ oauthWaitingForCallback ? '等待回调中' : '准备授权' }}
            </span>
          </div>

          <div class="oauth-field-list">
            <div class="oauth-field">
              <span class="oauth-field-label">授权链接</span>
              <n-input
                :value="oauthLogin?.auth_url ?? ''"
                type="textarea"
                readonly
                :autosize="{ minRows: 2, maxRows: 4 }"
                placeholder="生成授权链接后显示"
              />
            </div>
            <div class="oauth-field">
              <span class="oauth-field-label">回调地址</span>
              <n-input
                :value="oauthLogin?.redirect_uri ?? ''"
                readonly
                placeholder="本地回调监听地址"
              />
            </div>
          </div>

          <div class="oauth-actions">
            <n-button
              type="primary"
              :disabled="!oauthLogin"
              :loading="oauthOpening"
              @click="handleOpenOAuthLoginUrl"
            >
              打开授权页
            </n-button>
            <n-button secondary :loading="oauthCancelling" @click="handleCancelOAuthLogin">
              取消登录
            </n-button>
          </div>
        </section>

        <section class="oauth-panel oauth-panel-muted">
          <div class="oauth-panel-head">
            <div>
              <h4>手动回调兜底</h4>
              <p>浏览器未自动跳回时，将地址栏中的完整回调链接粘贴到下方。</p>
            </div>
          </div>

          <div v-if="oauthWaitingForCallback" class="oauth-status-card">
            已启动本地回调监听。若浏览器没有自动回跳，可直接复制回调链接到下方继续完成登录。
          </div>

          <div class="oauth-field">
            <span class="oauth-field-label">手动回调链接</span>
            <n-input
              v-model:value="oauthCallbackUrl"
              type="textarea"
              :rows="3"
              placeholder="http://localhost:1455/auth/callback?code=..."
            />
          </div>
          <n-button
            block
            secondary
            type="primary"
            :disabled="!oauthLogin"
            :loading="oauthSubmittingCallback"
            @click="handleSubmitOAuthCallback"
          >
            使用回调链接完成登录
          </n-button>
        </section>
      </div>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, h, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { storeToRefs } from 'pinia'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { useMessage, useDialog } from 'naive-ui'
import type { DropdownOption } from 'naive-ui'
import { useAccountStore } from '@/stores/account'
import { accountService, authService, usageService } from '@/services'
import { AUTH_TYPE_LABELS } from '@/types'
import type {
  Account,
  CodexShortConversationResult,
  OAuthCallbackFinishedEvent,
  OAuthLoginResult,
  PreparedOAuthLogin,
} from '@/types'
import AccountCard from '@/components/account/AccountCard.vue'
import CreateAccountModal from '@/components/account/CreateAccountModal.vue'
import { resolveAccountDisplayName } from '@/utils/account-display'

const router = useRouter()
const message = useMessage()
const dialog = useDialog()
const accountStore = useAccountStore()
const { accounts, loading, checkingStatus, activeAccount } = storeToRefs(accountStore)

type WarmupProgressMode = 'single' | 'batch'
type WarmupProgressState = 'pending' | 'running' | 'success' | 'failed'

interface WarmupProgressEntry {
  accountId: string
  accountName: string
  state: WarmupProgressState
  detail: string
}

const FULL_FIVE_HOUR_UNUSED_THRESHOLD = 0.000_001

const searchQuery = ref('')
const showCreateModal = ref(false)
const showOAuthModal = ref(false)
const exportLoading = ref(false)
const importLoading = ref(false)
const checkingAll = ref(false)
const syncingLocalAuth = ref(false)
const triggeringConversation = ref(false)
const refreshingTokenAccounts = ref<Set<string>>(new Set())
const triggeringConversationAccounts = ref<Set<string>>(new Set())
const warmupProgressMode = ref<WarmupProgressMode>('batch')
const warmupProgressEntries = ref<WarmupProgressEntry[]>([])
const oauthPreparing = ref(false)
const oauthOpening = ref(false)
const oauthWaitingForCallback = ref(false)
const oauthSubmittingCallback = ref(false)
const oauthCancelling = ref(false)
const oauthLogin = ref<PreparedOAuthLogin | null>(null)
const oauthCallbackUrl = ref('')
let oauthCallbackUnlisten: UnlistenFn | null = null

const hasAccounts = computed(() => accounts.value.length > 0)
const warmupProgressVisible = computed(() => warmupProgressEntries.value.length > 0)
const warmupExecutableCount = computed(() => warmupProgressEntries.value.length)
const warmupCompletedCount = computed(
  () =>
    warmupProgressEntries.value.filter(
      (entry) => entry.state === 'success' || entry.state === 'failed',
    ).length,
)
const warmupSuccessCount = computed(
  () => warmupProgressEntries.value.filter((entry) => entry.state === 'success').length,
)
const warmupFailedCount = computed(
  () => warmupProgressEntries.value.filter((entry) => entry.state === 'failed').length,
)
const warmupProgressPercentage = computed(() => {
  if (!warmupProgressEntries.value.length) {
    return 0
  }

  if (warmupExecutableCount.value === 0) {
    return 100
  }

  return Math.round((warmupCompletedCount.value / warmupExecutableCount.value) * 100)
})
const warmupProgressTitle = computed(() =>
  warmupProgressMode.value === 'single' ? '预热执行进度' : '批量预热进度'
)
const warmupProgressSummary = computed(() => {
  if (!warmupProgressEntries.value.length) {
    return ''
  }

  return `已完成 ${warmupCompletedCount.value}/${warmupExecutableCount.value} 个可执行账号。`
})

type AccountActionKey =
  | 'sync-local'
  | 'oauth-login'
  | 'trigger-conversation'
  | 'check-all'
  | 'import'
  | 'export'
  | 'create'

const accountActionOptions = computed<DropdownOption[]>(() => [
  {
    label: syncingLocalAuth.value ? '本地同步中' : '本地同步',
    key: 'sync-local',
    disabled: syncingLocalAuth.value,
    icon: () => renderDropdownIcon('M21 12a9 9 0 1 1-2.64-6.36M21 3v6h-6'),
  },
  {
    label: oauthPreparing.value || oauthOpening.value ? 'OAuth 登录中' : 'OAuth 登录',
    key: 'oauth-login',
    disabled: oauthPreparing.value || oauthOpening.value,
    icon: () =>
      renderDropdownIcon(
        'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06A1.65 1.65 0 0 0 15 19.4a1.65 1.65 0 0 0-1 .6 1.65 1.65 0 0 0-.33 1.82l.03.08a2 2 0 1 1-3.4 0l.03-.08A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.6 15a1.65 1.65 0 0 0-.6-1 1.65 1.65 0 0 0-1.82-.33l-.08.03a2 2 0 1 1 0-3.4l.08.03A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-.6 1.65 1.65 0 0 0 .33-1.82l-.03-.08a2 2 0 1 1 3.4 0l-.03.08A1.65 1.65 0 0 0 15 4.6a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 .6 1 1.65 1.65 0 0 0 1.82.33l.08-.03a2 2 0 1 1 0 3.4l-.08-.03A1.65 1.65 0 0 0 19.4 15z',
      ),
  },
  {
    label: triggeringConversation.value ? '预热中' : '一键预热',
    key: 'trigger-conversation',
    disabled: triggeringConversation.value || !hasAccounts.value,
    props: {
      title: '触发5小时倒计时',
    },
    icon: () => renderDropdownIcon('M21 15a4 4 0 0 1-4 4H7l-4 4V7a4 4 0 0 1 4-4h10a4 4 0 0 1 4 4v8z'),
  },
  {
    label: checkingAll.value ? '检测中' : '检测全部',
    key: 'check-all',
    disabled: checkingAll.value || !hasAccounts.value,
    icon: () => renderDropdownIcon('M20 12a8 8 0 1 1-2.34-5.66M20 4v6h-6'),
  },
  {
    type: 'divider',
    key: 'account-action-divider-1',
  },
  {
    label: importLoading.value ? '导入中' : '导入',
    key: 'import',
    disabled: importLoading.value,
    icon: () => renderDropdownIcon('M12 3v12M7 8l5-5 5 5M5 21h14'),
  },
  {
    label: exportLoading.value ? '导出中' : '导出',
    key: 'export',
    disabled: exportLoading.value || !hasAccounts.value,
    icon: () => renderDropdownIcon('M12 21V9M7 16l5 5 5-5M5 3h14'),
  },
  {
    type: 'divider',
    key: 'account-action-divider-2',
  },
  {
    label: '新增账号',
    key: 'create',
    icon: () => renderDropdownIcon('M12 5v14M5 12h14'),
  },
])

const filteredAccounts = computed(() => {
  let list = accounts.value
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    list = list.filter(
      (account) =>
        resolveAccountDisplayName(account).toLowerCase().includes(query) ||
        account.email?.toLowerCase().includes(query),
    )
  }
  return list
})

const SearchIcon = {
  render: () =>
    h('svg', { width: 14, height: 14, viewBox: '0 0 24 24', fill: 'none' }, [
      h('circle', { cx: 11, cy: 11, r: 8, stroke: 'currentColor', 'stroke-width': 2 }),
      h('path', {
        d: 'm21 21-4.35-4.35',
        stroke: 'currentColor',
        'stroke-width': 2,
        'stroke-linecap': 'round',
      }),
    ]),
}

function renderDropdownIcon(path: string) {
  return h(
    'svg',
    {
      width: 16,
      height: 16,
      viewBox: '0 0 24 24',
      fill: 'none',
    },
    h('path', {
      d: path,
      stroke: 'currentColor',
      'stroke-width': 1.8,
      'stroke-linecap': 'round',
      'stroke-linejoin': 'round',
    }),
  )
}

onMounted(() => {
  void registerOAuthCallbackListener()
})

onUnmounted(() => {
  oauthCallbackUnlisten?.()
  oauthCallbackUnlisten = null
  if (oauthLogin.value) {
    void authService.cancelOAuthLogin().catch((error) => {
      console.warn('取消 OAuth 登录失败', error)
    })
  }
})

function navigateToDetail(id: string) {
  accountStore.setActive(id)
  router.push({ name: 'AccountDetail', params: { id } })
}

function hasFullFiveHourQuota(account: Account): boolean {
  return Boolean(account.codex_usage_5h && account.codex_usage_5h.used_percent <= FULL_FIVE_HOUR_UNUSED_THRESHOLD)
}

// 批量预热会跨多次后端调用，前端需要保留每个账号的状态，用户才能知道具体卡在哪个账号。
function resetWarmupProgress(targetAccounts: Account[], mode: WarmupProgressMode) {
  warmupProgressMode.value = mode
  warmupProgressEntries.value = targetAccounts.map((account) => ({
    accountId: account.id,
    accountName: resolveAccountDisplayName(account),
    state: 'pending',
    detail: '等待执行',
  }))
}

function updateWarmupProgressEntry(
  accountId: string,
  state: WarmupProgressState,
  detail: string,
) {
  warmupProgressEntries.value = warmupProgressEntries.value.map((entry) =>
    entry.accountId === accountId ? { ...entry, state, detail } : entry,
  )
}

function clearWarmupProgress() {
  if (triggeringConversation.value) {
    return
  }

  warmupProgressEntries.value = []
}

function resolveWarmupStateLabel(state: WarmupProgressState): string {
  switch (state) {
    case 'pending':
      return '等待中'
    case 'running':
      return '执行中'
    case 'success':
      return '已完成'
    case 'failed':
      return '失败'
  }
}

function resolveWarmupWorkspaceDescription(result: CodexShortConversationResult): string {
  if (result.working_directory?.trim()) {
    return result.working_directory.trim()
  }

  if (result.working_directory_source === 'process_cwd') {
    return '应用安装目录'
  }

  return '未显式指定目录'
}

async function handleCheckStatus(id: string) {
  try {
    await accountStore.checkAccountStatus(id)
    message.success('状态检测完成')
  } catch {
    message.error('状态检测失败')
  }
}

async function handleRefreshToken(id: string) {
  if (refreshingTokenAccounts.value.has(id)) {
    return
  }

  refreshingTokenAccounts.value = new Set([...refreshingTokenAccounts.value, id])
  try {
    const result = await authService.refreshToken(id)
    await accountStore.loadAccounts()
    message.success(result.message || 'Token 已刷新')
  } catch (error) {
    message.error(getErrorMessage(error, '刷新 Token 失败'))
  } finally {
    const nextRefreshingAccounts = new Set(refreshingTokenAccounts.value)
    nextRefreshingAccounts.delete(id)
    refreshingTokenAccounts.value = nextRefreshingAccounts
  }
}

async function handleCheckAll() {
  checkingAll.value = true
  try {
    await accountStore.checkAllStatus()
    message.success('全部账号检测完成')
  } catch {
    message.error('检测失败')
  } finally {
    checkingAll.value = false
  }
}

function handleAccountActionSelect(key: string | number) {
  switch (key as AccountActionKey) {
    case 'sync-local':
      void handleSyncLocalAuth()
      break
    case 'oauth-login':
      void handlePrepareOAuthLogin()
      break
    case 'trigger-conversation':
      void handleTriggerConversation()
      break
    case 'check-all':
      void handleCheckAll()
      break
    case 'import':
      void handleImportAccounts()
      break
    case 'export':
      void handleExportAccounts()
      break
    case 'create':
      showCreateModal.value = true
      break
  }
}

async function handleSwitchAccount(id: string) {
  try {
    await accountStore.switchAccount(id)
    message.success('已切换当前账号')
  } catch (error) {
    message.error(getErrorMessage(error, '切换账号失败'))
  }
}

async function handleTriggerConversation(accountId?: string) {
  if (triggeringConversation.value) return
  const isBatchWarmup = !accountId
  const previousActiveAccountId = isBatchWarmup ? activeAccount.value?.id ?? null : null

  const targetAccounts = accountId
    ? accounts.value.filter((account) => account.id === accountId)
    : [...accounts.value]

  if (targetAccounts.length === 0) {
    message.warning('没有可执行一键预热的账号')
    return
  }

  const executableAccounts = targetAccounts.filter((account) => hasFullFiveHourQuota(account))

  resetWarmupProgress(executableAccounts, accountId ? 'single' : 'batch')

  if (executableAccounts.length === 0) {
    message.warning(
      accountId
        ? '该账号当前没有可执行的一键预热额度'
        : '当前没有 5 小时剩余额度为 100% 的账号可执行一键预热',
    )
    return
  }

  triggeringConversation.value = true
  try {
    for (const account of executableAccounts) {
      updateWarmupProgressEntry(account.id, 'running', '正在发送最短对话预热')
      triggeringConversationAccounts.value = new Set([
        ...triggeringConversationAccounts.value,
        account.id,
      ])

      try {
        const result = await usageService.triggerCodexShortConversation(account.id)
        updateWarmupProgressEntry(
          account.id,
          'success',
          `${result.model} 预热完成 · 工作区：${resolveWarmupWorkspaceDescription(result)}`,
        )

        if (accountId) {
          message.success(`已通过「${result.account_name}」完成 ${result.model} 一键预热`)
        }
      } catch (error) {
        const errorMessage = getErrorMessage(error, '一键预热失败')
        updateWarmupProgressEntry(account.id, 'failed', errorMessage)

        if (accountId) {
          message.error(errorMessage)
        }
      } finally {
        const nextTriggeringAccounts = new Set(triggeringConversationAccounts.value)
        nextTriggeringAccounts.delete(account.id)
        triggeringConversationAccounts.value = nextTriggeringAccounts
      }
    }

    try {
      await accountStore.loadAccounts()
    } catch {
      message.warning('预热结果已返回，但刷新账号列表失败')
    }

    if (isBatchWarmup) {
      await restorePreviousActiveAccount(previousActiveAccountId)

      const successCount = warmupSuccessCount.value
      const failedCount = warmupFailedCount.value
      if (failedCount > 0) {
        message.warning(`批量预热完成，成功 ${successCount} 个，失败 ${failedCount} 个`)
      } else {
        message.success(`批量预热完成，成功 ${successCount} 个`)
        warmupProgressEntries.value = []
      }
    }
  } finally {
    triggeringConversation.value = false
  }
}

// 批量预热会依次写回多个账号到默认 auth.json，结束后恢复用户开始前的当前账号，避免后台预热改变后续 CLI 使用上下文。
async function restorePreviousActiveAccount(previousAccountId: string | null) {
  if (!previousAccountId) {
    return
  }

  const previousAccountExists = accounts.value.some((account) => account.id === previousAccountId)
  if (!previousAccountExists) {
    return
  }

  try {
    await accountStore.switchAccount(previousAccountId)
  } catch (error) {
    message.warning(getErrorMessage(error, '批量预热已结束，但恢复原当前账号失败'))
  }
}

function handleDelete(account: Account) {
  const displayName = resolveAccountDisplayName(account)
  dialog.warning({
    title: '删除账号',
    content: `确定要删除账号「${displayName}」吗？此操作不可撤销，相关数据将一并清除。`,
    positiveText: '确认删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      await accountStore.deleteAccount(account.id)
      message.success('账号已删除')
    },
  })
}

function handleCreated(account: Account) {
  showCreateModal.value = false
  message.success(`账号「${resolveAccountDisplayName(account)}」创建成功`)
}

function getErrorMessage(error: unknown, fallback: string) {
  if (error instanceof Error && error.message) {
    return error.message.replace(/^Invalid input:\s*/i, '').trim() || fallback
  }
  if (typeof error === 'string' && error) {
    return error.replace(/^Invalid input:\s*/i, '').trim() || fallback
  }
  return fallback
}

async function handleExportAccount(account: Account) {
  exportLoading.value = true
  try {
    const outputPath = await selectExportPath(
      `${buildSafeFileName(resolveAccountDisplayName(account), 'account')}-auth.json`,
      [{ name: 'auth.json', extensions: ['json'] }],
    )
    if (!outputPath) return

    await accountService.exportAccountAuthFile(account.id, outputPath)
    message.success('已导出 auth.json，文件内包含明文凭证')
  } catch (error) {
    message.error(getErrorMessage(error, '导出 auth.json 失败'))
  } finally {
    exportLoading.value = false
  }
}

async function handleExportAccounts() {
  exportLoading.value = true
  try {
    const outputPath = await selectExportPath('codex-accounts-auth.zip', [
      { name: 'ZIP 压缩包', extensions: ['zip'] },
    ])
    if (!outputPath) return

    const result = await accountService.exportAccounts(outputPath)
    const skippedText = result.failed_count > 0 ? `，${result.failed_count} 个账号未导出` : ''
    message.success(`已导出 ${result.exported_count} 个 auth.json${skippedText}`)
  } catch (error) {
    message.error(getErrorMessage(error, '批量导出失败'))
  } finally {
    exportLoading.value = false
  }
}

async function handleImportAccounts() {
  importLoading.value = true
  try {
    const inputPath = await selectImportPath()
    if (!inputPath) return

    const result = await accountService.importAccounts(inputPath)
    await accountStore.loadAccounts()
    const skippedText = result.skipped_count > 0 ? `，跳过 ${result.skipped_count} 个非 JSON 条目` : ''
    const failedText = result.failed_count > 0 ? `，${result.failed_count} 个条目失败` : ''
    message.success(`已导入或更新 ${result.imported_count} 个账号${skippedText}${failedText}`)
  } catch (error) {
    message.error(getErrorMessage(error, '导入失败'))
  } finally {
    importLoading.value = false
  }
}

async function selectExportPath(
  defaultPath: string,
  filters: Array<{ name: string; extensions: string[] }>,
): Promise<string | null> {
  const { save } = await import('@tauri-apps/plugin-dialog')
  const selectedPath = await save({
    defaultPath,
    filters,
  })

  return selectedPath || null
}

async function selectImportPath(): Promise<string | null> {
  const { open } = await import('@tauri-apps/plugin-dialog')
  const selectedPath = await open({
    multiple: false,
    filters: [
      {
        name: '认证文件',
        extensions: ['json', 'zip'],
      },
    ],
  })

  return typeof selectedPath === 'string' ? selectedPath : null
}

function buildSafeFileName(value: string, fallback: string): string {
  const normalized = value
    .trim()
    .replace(/[<>:"/\\|?*\u0000-\u001F]/g, '_')
    .replace(/[. ]+$/g, '')
    .slice(0, 80)

  return normalized || fallback
}

async function handleSyncLocalAuth() {
  syncingLocalAuth.value = true
  try {
    const result = await accountStore.syncLocalAuthFile()
    if (result.codex_usage_error) {
      message.success(`已同步账号「${result.account_name}」，资料稍后重试`)
    } else if (result.codex_plan_type) {
      message.success(`已同步账号「${result.account_name}」(${result.codex_plan_type})`)
    } else {
      message.success(`已同步账号「${result.account_name}」(${AUTH_TYPE_LABELS[result.auth_type]})`)
    }
  } catch (error) {
    message.error(getErrorMessage(error, '本地账号同步失败'))
  } finally {
    syncingLocalAuth.value = false
  }
}

async function registerOAuthCallbackListener() {
  try {
    oauthCallbackUnlisten = await listen<OAuthCallbackFinishedEvent>(
      'oauth-callback-finished',
      (event) => {
        void handleOAuthCallbackFinished(event.payload)
      },
    )
  } catch (error) {
    console.warn('OAuth 回调事件监听不可用', error)
  }
}

async function handlePrepareOAuthLogin() {
  if (oauthPreparing.value) return

  oauthPreparing.value = true
  try {
    const prepared = await authService.prepareOAuthLogin()
    oauthLogin.value = prepared
    oauthCallbackUrl.value = ''
    oauthWaitingForCallback.value = true
    showOAuthModal.value = true
    await handleOpenOAuthLoginUrl()
  } catch (error) {
    resetOAuthLoginState()
    message.error(getErrorMessage(error, 'OAuth 登录初始化失败'))
  } finally {
    oauthPreparing.value = false
  }
}

async function handleOpenOAuthLoginUrl() {
  if (!oauthLogin.value) {
    message.warning('请先生成 OAuth 授权链接')
    return
  }

  oauthOpening.value = true
  try {
    await authService.openOAuthLoginUrl(oauthLogin.value.auth_url)
    oauthWaitingForCallback.value = true
  } catch (error) {
    message.warning(getErrorMessage(error, '无法打开系统浏览器，请手动复制授权链接'))
  } finally {
    oauthOpening.value = false
  }
}

async function handleSubmitOAuthCallback() {
  const callbackUrl = oauthCallbackUrl.value.trim()
  if (!callbackUrl) {
    message.warning('请粘贴 OAuth 回调链接')
    return
  }

  oauthSubmittingCallback.value = true
  try {
    const result = await authService.completeOAuthCallbackLogin(callbackUrl)
    await applyOAuthLoginResult(result)
  } catch (error) {
    message.error(getErrorMessage(error, 'OAuth 登录失败'))
  } finally {
    oauthSubmittingCallback.value = false
  }
}

async function handleOAuthCallbackFinished(payload: OAuthCallbackFinishedEvent) {
  if (payload.result) {
    await applyOAuthLoginResult(payload.result)
    return
  }

  resetOAuthLoginState()
  message.error(payload.error ?? 'OAuth 登录未返回账号结果')
}

async function applyOAuthLoginResult(result: OAuthLoginResult) {
  await accountStore.loadAccounts()
  resetOAuthLoginState()
  message.success(`已导入账号「${result.account_name}」(${AUTH_TYPE_LABELS[result.auth_type]})`)
}

function handleOAuthModalVisibleChange(visible: boolean) {
  if (visible) {
    showOAuthModal.value = true
    return
  }

  void handleCancelOAuthLogin()
}

async function handleCancelOAuthLogin() {
  const hasPendingLogin = Boolean(oauthLogin.value) || oauthWaitingForCallback.value
  if (!hasPendingLogin) {
    resetOAuthLoginState()
    return
  }

  oauthCancelling.value = true
  try {
    await authService.cancelOAuthLogin()
  } catch (error) {
    message.warning(getErrorMessage(error, '取消 OAuth 登录失败'))
  } finally {
    resetOAuthLoginState()
  }
}

function resetOAuthLoginState() {
  showOAuthModal.value = false
  oauthLogin.value = null
  oauthCallbackUrl.value = ''
  oauthWaitingForCallback.value = false
  oauthOpening.value = false
  oauthSubmittingCallback.value = false
  oauthCancelling.value = false
}

</script>

<style scoped>
.panel-heading {
  font-size: 18px;
}

.account-grid-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  flex-wrap: wrap;
}

.account-section {
  min-width: 0;
  flex-shrink: 0;
}

.account-header-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  min-width: 0;
  flex: 1;
}

.account-action-content {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  line-height: 1;
}

.account-action-trigger {
  height: 34px;
  min-width: 112px;
  background: var(--app-surface);
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.1);
}

.search-row {
  width: min(360px, 100%);
}

.search-row :deep(.n-input) {
  height: 34px;
}

.search-row :deep(.n-input__input-el) {
  font-size: 13px;
}

.warmup-progress-panel {
  display: grid;
  gap: 12px;
  padding: 16px 18px;
  border-radius: 22px;
  border: 1px solid rgba(0, 113, 227, 0.12);
  background:
    linear-gradient(135deg, rgba(0, 113, 227, 0.08), rgba(0, 113, 227, 0.02) 54%),
    var(--app-surface-muted);
}

.warmup-progress-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.warmup-progress-copy {
  display: grid;
  gap: 4px;
}

.warmup-progress-copy h3 {
  margin: 0;
  font-size: 15px;
  line-height: 1.3;
  font-family: var(--font-display);
}

.warmup-progress-copy p {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--app-ink-secondary);
}

.warmup-progress-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.warmup-progress-chip {
  display: inline-flex;
  align-items: center;
  min-height: 24px;
  padding: 0 10px;
  border-radius: 999px;
  background: rgba(29, 29, 31, 0.06);
  color: var(--app-ink-secondary);
  font-size: 12px;
  line-height: 1;
}

.warmup-progress-list {
  display: grid;
  gap: 8px;
  max-height: 240px;
  overflow: auto;
  padding-right: 4px;
}

.warmup-progress-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  border-radius: 16px;
  background: rgba(255, 255, 255, 0.72);
  border: 1px solid rgba(29, 29, 31, 0.06);
}

.warmup-progress-item.state-running {
  border-color: rgba(0, 113, 227, 0.18);
}

.warmup-progress-item.state-success {
  border-color: rgba(52, 199, 89, 0.18);
}

.warmup-progress-item.state-failed {
  border-color: rgba(208, 48, 80, 0.18);
}

.warmup-progress-item-copy {
  display: grid;
  gap: 3px;
  min-width: 0;
}

.warmup-progress-item-copy strong {
  font-size: 13px;
  line-height: 1.4;
}

.warmup-progress-item-copy span {
  font-size: 12px;
  line-height: 1.5;
  color: var(--app-ink-secondary);
  word-break: break-word;
}

.warmup-progress-item-status {
  flex-shrink: 0;
  font-size: 12px;
  line-height: 1.33;
  color: var(--app-ink-secondary);
}

.warmup-progress-item.state-running .warmup-progress-item-status {
  color: var(--app-blue);
}

.warmup-progress-item.state-success .warmup-progress-item-status {
  color: #248a3d;
}

.warmup-progress-item.state-failed .warmup-progress-item-status {
  color: #c43b57;
}

.account-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(280px, 100%), 1fr));
  gap: 14px;
  width: 100%;
  min-width: 0;
}

.inline-empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 160px;
  border-radius: 20px;
  background: var(--app-surface-muted);
  color: var(--app-ink-secondary);
  font-size: 13px;
}

.oauth-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.oauth-modal-layout {
  display: grid;
  gap: 16px;
}

.oauth-hero-card,
.oauth-panel {
  display: grid;
  gap: 14px;
  padding: 18px;
  border-radius: 22px;
  background: var(--app-surface-muted);
}

.oauth-hero-card {
  background:
    linear-gradient(135deg, rgba(0, 113, 227, 0.1), rgba(0, 113, 227, 0.02) 58%),
    var(--app-surface-muted);
}

.oauth-hero-copy {
  display: grid;
  gap: 6px;
}

.oauth-eyebrow,
.oauth-field-label,
.oauth-step-index {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-blue);
}

.oauth-hero-copy h3,
.oauth-panel-head h4,
.oauth-step-card strong {
  margin: 0;
  font-family: var(--font-display);
  color: var(--app-ink);
}

.oauth-hero-copy h3 {
  font-size: 24px;
  line-height: 1.18;
}

.oauth-hero-copy p,
.oauth-panel-head p,
.oauth-step-card p,
.oauth-status-card {
  margin: 0;
  font-size: 13px;
  line-height: 1.5;
  color: var(--app-ink-secondary);
}

.oauth-step-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}

.oauth-step-card {
  display: grid;
  gap: 6px;
  padding: 14px;
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.7);
  border: 1px solid rgba(0, 113, 227, 0.08);
}

.oauth-panel-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.oauth-status-pill {
  display: inline-flex;
  align-items: center;
  min-height: 28px;
  padding: 0 12px;
  border-radius: var(--app-radius-control);
  background: rgba(29, 29, 31, 0.06);
  color: var(--app-ink-secondary);
  font-size: 12px;
  line-height: 1.33;
  white-space: nowrap;
}

.oauth-status-pill.active {
  background: rgba(52, 199, 89, 0.14);
  color: #248a3d;
}

.oauth-field-list,
.oauth-field {
  display: grid;
  gap: 8px;
}

.oauth-status-card {
  padding: 12px 14px;
  border-radius: 16px;
  background: rgba(52, 199, 89, 0.12);
  color: #248a3d;
}

.oauth-panel-muted {
  background: var(--app-surface);
  border: 1px solid rgba(29, 29, 31, 0.08);
}

@media (max-width: 768px) {
  .account-grid {
    grid-template-columns: 1fr;
  }

  .warmup-progress-header,
  .warmup-progress-item {
    flex-direction: column;
    align-items: flex-start;
  }

  .oauth-step-grid {
    grid-template-columns: 1fr;
  }

  .oauth-panel-head {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
