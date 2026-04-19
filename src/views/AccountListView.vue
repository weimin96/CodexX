<template>
  <div class="app-page">
    <section class="page-hero">
      <div class="page-hero-copy">
        <h1 class="page-title">账号管理</h1>
        <div class="page-hero-actions">
          <n-dropdown
            trigger="click"
            :options="accountActionOptions"
            @select="handleAccountActionSelect"
          >
            <n-button
              circle
              secondary
              class="account-action-trigger"
              title="账号操作"
              aria-label="账号操作"
            >
              <template #icon>
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                  <path
                    d="M5 12h.01M12 12h.01M19 12h.01"
                    stroke="currentColor"
                    stroke-width="2.4"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </template>
            </n-button>
          </n-dropdown>
        </div>
      </div>
    </section>

    <section v-if="loading" class="surface-panel empty-panel">
      <n-spin size="medium" />
      <p>正在加载账号。</p>
    </section>

    <section v-else-if="!hasAccounts" class="surface-panel empty-panel">
      <div class="empty-illustration">
        <svg width="68" height="68" viewBox="0 0 24 24" fill="none">
          <path
            d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"
            stroke="currentColor"
            stroke-width="1.5"
          />
          <circle cx="9" cy="7" r="4" stroke="currentColor" stroke-width="1.5" />
          <path
            d="M23 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"
            stroke="currentColor"
            stroke-width="1.5"
          />
        </svg>
      </div>
      <p>还没有添加任何账号。</p>
      <n-button type="primary" @click="showCreateModal = true">添加第一个账号</n-button>
    </section>

    <section v-else class="surface-panel section-grid account-section">
      <div class="account-grid-header">
        <div>
          <h2 class="panel-heading">账号信息</h2>
        </div>
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
      </div>

      <div v-if="filteredAccounts.length === 0" class="inline-empty-state">
        <p>没有匹配的账号。</p>
      </div>

      <div v-else class="account-grid">
        <AccountCard
          v-for="account in filteredAccounts"
          :key="account.id"
          :account="account"
          :checking="checkingStatus.has(account.id)"
          @detail="navigateToDetail(account.id)"
          @check="handleCheckStatus(account.id)"
          @set-default="handleSetDefault(account.id)"
          @delete="handleDelete(account)"
        />
      </div>
    </section>

    <CreateAccountModal v-model:show="showCreateModal" @created="handleCreated" />

    <n-modal v-model:show="showExportModal" preset="card" title="导出账号" style="width: 420px;">
      <n-form>
        <n-form-item label="加密密码">
          <n-input
            v-model:value="exportPassword"
            type="password"
            placeholder="请输入导出密码"
            show-password-on="click"
          />
        </n-form-item>
        <n-alert type="warning" style="margin-bottom: 12px;">
          导出文件经过 AES-256-GCM 加密，请妥善保管密码。
        </n-alert>
        <n-button type="primary" block :loading="exportLoading" @click="doExport">
          确认导出
        </n-button>
      </n-form>
    </n-modal>

    <n-modal v-model:show="showImportModal" preset="card" title="导入账号" style="width: 460px;">
      <n-form>
        <n-form-item label="加密文件内容">
          <n-input
            v-model:value="importData"
            type="textarea"
            :rows="4"
            placeholder="粘贴导出的加密内容..."
          />
        </n-form-item>
        <n-form-item label="解密密码">
          <n-input
            v-model:value="importPassword"
            type="password"
            show-password-on="click"
            placeholder="请输入导出时的密码"
          />
        </n-form-item>
        <n-button type="primary" block :loading="importLoading" @click="doImport">
          确认导入
        </n-button>
      </n-form>
    </n-modal>

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
import { accountService, authService } from '@/services'
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
const showExportModal = ref(false)
const showImportModal = ref(false)
const showOAuthModal = ref(false)
const exportPassword = ref('')
const importData = ref('')
const importPassword = ref('')
const exportLoading = ref(false)
const importLoading = ref(false)
const checkingAll = ref(false)
const syncingLocalAuth = ref(false)
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
  | 'check-all'
  | 'import'
  | 'export'
  | 'create'

const accountActionOptions = computed<DropdownOption[]>(() => [
  {
    label: syncingLocalAuth.value ? '本地同步中' : '本地同步',
    key: 'sync-local',
    disabled: syncingLocalAuth.value,
  },
  {
    label: oauthPreparing.value || oauthOpening.value ? 'OAuth 登录中' : 'OAuth 登录',
    key: 'oauth-login',
    disabled: oauthPreparing.value || oauthOpening.value,
  },
  {
    label: checkingAll.value ? '检测中' : '检测全部',
    key: 'check-all',
    disabled: checkingAll.value || !hasAccounts.value,
  },
  {
    type: 'divider',
    key: 'account-action-divider-1',
  },
  {
    label: '导入',
    key: 'import',
  },
  {
    label: '导出',
    key: 'export',
  },
  {
    type: 'divider',
    key: 'account-action-divider-2',
  },
  {
    label: '新增账号',
    key: 'create',
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
    case 'check-all':
      void handleCheckAll()
      break
    case 'import':
      showImportModal.value = true
      break
    case 'export':
      handleExport()
      break
    case 'create':
      showCreateModal.value = true
      break
  }
}

async function handleSetDefault(id: string) {
  await accountStore.switchAccount(id)
  message.success('已设为默认账号')
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

function handleExport() {
  exportPassword.value = ''
  showExportModal.value = true
}

function getErrorMessage(error: unknown, fallback: string) {
  if (error instanceof Error && error.message) return error.message
  if (typeof error === 'string' && error) return error
  return fallback
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

async function doExport() {
  if (!exportPassword.value) {
    message.warning('请输入导出密码')
    return
  }
  exportLoading.value = true
  try {
    const encrypted = await accountService.exportAccounts(exportPassword.value)
    await navigator.clipboard.writeText(encrypted)
    showExportModal.value = false
    message.success('导出成功，已复制到剪贴板')
  } catch {
    message.error('导出失败')
  } finally {
    exportLoading.value = false
  }
}

async function doImport() {
  if (!importData.value || !importPassword.value) {
    message.warning('请填写完整信息')
    return
  }
  importLoading.value = true
  try {
    const count = await accountService.importAccounts(importData.value.trim(), importPassword.value)
    await accountStore.loadAccounts()
    showImportModal.value = false
    message.success(`成功导入 ${count} 个账号`)
  } catch {
    message.error('导入失败：密码错误或数据损坏')
  } finally {
    importLoading.value = false
  }
}
</script>

<style scoped>
.page-title {
  font-size: clamp(21px, 3.1vw, 28px);
  line-height: 1.08;
}

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

.account-action-trigger {
  color: var(--app-hero-ink);
}

.account-section {
  min-width: 0;
  flex-shrink: 0;
}

.search-row {
  width: min(360px, 100%);
}

.search-row :deep(.n-input__input-el) {
  font-size: 13px;
}

.empty-illustration {
  width: 92px;
  height: 92px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--app-surface-muted);
  color: var(--app-ink-tertiary);
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
