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
              placeholder="搜索账号名称、邮箱、组织..."
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
              title="账号操作"
              aria-label="账号操作"
            >
              <span class="account-action-content">
                <span>账号操作</span>
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

      <div v-if="!hasAccounts" class="inline-empty-state">
        <p>还没有添加任何账号，请通过账号操作添加、导入或同步账号。</p>
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
          :triggering-conversation="triggeringConversationAccounts.has(account.id)"
          @detail="navigateToDetail(account.id)"
          @check="handleCheckStatus(account.id)"
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
      style="width: 560px;"
      @update:show="handleOAuthModalVisibleChange"
    >
      <n-space vertical size="large">
        <n-alert type="info">
          将通过系统浏览器打开 OpenAI 授权页，应用只监听 127.0.0.1 本地回调并加密保存登录结果。
        </n-alert>

        <n-form>
          <n-form-item label="授权链接">
            <n-input
              :value="oauthLogin?.auth_url ?? ''"
              type="textarea"
              readonly
              :autosize="{ minRows: 2, maxRows: 4 }"
              placeholder="生成授权链接后显示"
            />
          </n-form-item>
          <n-form-item label="回调地址">
            <n-input
              :value="oauthLogin?.redirect_uri ?? ''"
              readonly
              placeholder="本地回调监听地址"
            />
          </n-form-item>
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
        </n-form>

        <n-alert v-if="oauthWaitingForCallback" type="success">
          已启动本地回调监听。浏览器授权完成后通常会自动回到应用；如果浏览器没有跳回，请复制浏览器地址栏中的回调链接到下方。
        </n-alert>

        <n-form>
          <n-form-item label="手动回调链接">
            <n-input
              v-model:value="oauthCallbackUrl"
              type="textarea"
              :rows="3"
              placeholder="http://localhost:1455/auth/callback?code=..."
            />
          </n-form-item>
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
        </n-form>
      </n-space>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, h, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { useMessage, useDialog } from 'naive-ui'
import type { DropdownOption } from 'naive-ui'
import { useAccountStore } from '@/stores/account'
import { accountService, authService, usageService } from '@/services'
import { AUTH_TYPE_LABELS } from '@/types'
import type {
  Account,
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
const { accounts, loading, checkingStatus } = storeToRefs(accountStore)

const searchQuery = ref('')
const showCreateModal = ref(false)
const showOAuthModal = ref(false)
const exportLoading = ref(false)
const importLoading = ref(false)
const checkingAll = ref(false)
const syncingLocalAuth = ref(false)
const triggeringConversation = ref(false)
const triggeringConversationAccounts = ref<Set<string>>(new Set())
const oauthPreparing = ref(false)
const oauthOpening = ref(false)
const oauthWaitingForCallback = ref(false)
const oauthSubmittingCallback = ref(false)
const oauthCancelling = ref(false)
const oauthLogin = ref<PreparedOAuthLogin | null>(null)
const oauthCallbackUrl = ref('')
let oauthCallbackUnlisten: UnlistenFn | null = null

const hasAccounts = computed(() => accounts.value.length > 0)

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
    label: triggeringConversation.value ? '触发中' : '触发对话',
    key: 'trigger-conversation',
    disabled: triggeringConversation.value || !hasAccounts.value,
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
        account.email?.toLowerCase().includes(query) ||
        account.organization?.toLowerCase().includes(query),
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

async function handleCheckStatus(id: string) {
  try {
    await accountStore.checkAccountStatus(id)
    message.success('状态检测完成')
  } catch {
    message.error('状态检测失败')
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

  triggeringConversation.value = true
  if (accountId) {
    triggeringConversationAccounts.value = new Set([...triggeringConversationAccounts.value, accountId])
  }

  try {
    const result = await usageService.triggerCodexShortConversation(accountId)
    await accountStore.loadAccounts()
    message.success(`已通过「${result.account_name}」触发 ${result.model} 对话`)
  } catch (error) {
    message.error(getErrorMessage(error, '触发对话失败'))
  } finally {
    if (accountId) {
      const nextTriggeringAccounts = new Set(triggeringConversationAccounts.value)
      nextTriggeringAccounts.delete(accountId)
      triggeringConversationAccounts.value = nextTriggeringAccounts
    }
    triggeringConversation.value = false
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
  if (error instanceof Error && error.message) return error.message
  if (typeof error === 'string' && error) return error
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

@media (max-width: 768px) {
  .account-grid {
    grid-template-columns: 1fr;
  }
}
</style>
