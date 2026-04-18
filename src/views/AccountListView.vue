<template>
  <div class="view-container">
    <!-- 头部 -->
    <div class="view-header">
      <div>
        <h1 class="view-title">账号列表</h1>
        <p class="view-sub">管理所有 Codex 账号 · {{ totalAccounts }} 个账号</p>
      </div>
      <div class="header-actions">
        <n-button
          :loading="checkingAll"
          quaternary
          size="small"
          @click="handleCheckAll"
        >
          <template #icon>
            <n-icon><RefreshIcon /></n-icon>
          </template>
          检测全部
        </n-button>
        <n-button
          quaternary
          size="small"
          :loading="syncingLocalAuth"
          @click="handleSyncLocalAuth"
        >本地同步</n-button>
        <n-button
          quaternary
          size="small"
          :loading="oauthPreparing || oauthOpening"
          @click="handlePrepareOAuthLogin"
        >OAuth 登录</n-button>
        <n-button
          quaternary
          size="small"
          @click="showImportModal = true"
        >导入</n-button>
        <n-button
          quaternary
          size="small"
          @click="handleExport"
        >导出</n-button>
        <n-button type="primary" size="small" @click="showCreateModal = true">
          <template #icon>
            <n-icon><PlusIcon /></n-icon>
          </template>
          新增账号
        </n-button>
      </div>
    </div>

    <!-- 状态汇总 -->
    <div class="status-bar">
      <div
        v-for="(label, key) in STATUS_LABELS"
        :key="key"
        class="status-chip"
        :class="{ active: filterStatus === key }"
        @click="filterStatus = filterStatus === key ? null : key as AccountStatus"
      >
        <StatusDot :status="key as AccountStatus" />
        <span>{{ label }}</span>
        <span class="chip-count">{{ accountsByStatus[key]?.length ?? 0 }}</span>
      </div>
    </div>

    <!-- 搜索 -->
    <div class="search-row">
      <n-input
        v-model:value="searchQuery"
        placeholder="搜索账号名称、邮箱、组织..."
        clearable
        size="small"
      >
        <template #prefix>
          <n-icon><SearchIcon /></n-icon>
        </template>
      </n-input>
    </div>

    <!-- 账号网格 -->
    <div v-if="loading" class="loading-state">
      <n-spin size="medium" />
    </div>

    <div v-else-if="filteredAccounts.length === 0" class="empty-state">
      <svg width="64" height="64" viewBox="0 0 24 24" fill="none" opacity="0.3">
        <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" stroke="#8b949e" stroke-width="1.5"/>
        <circle cx="9" cy="7" r="4" stroke="#8b949e" stroke-width="1.5"/>
        <path d="M23 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75" stroke="#8b949e" stroke-width="1.5"/>
      </svg>
      <p>{{ searchQuery || filterStatus ? '没有匹配的账号' : '还没有添加任何账号' }}</p>
      <n-button v-if="!searchQuery && !filterStatus" type="primary" size="small" @click="showCreateModal = true">
        添加第一个账号
      </n-button>
    </div>

    <div v-else class="account-grid">
      <AccountCard
        v-for="account in filteredAccounts"
        :key="account.id"
        :account="account"
        :checking="checkingStatus.has(account.id)"
        @click="navigateToDetail(account.id)"
        @check="handleCheckStatus(account.id)"
        @set-default="handleSetDefault(account.id)"
        @delete="handleDelete(account)"
      />
    </div>

    <!-- 创建弹窗 -->
    <CreateAccountModal
      v-model:show="showCreateModal"
      @created="handleCreated"
    />

    <!-- 导出弹窗 -->
    <n-modal v-model:show="showExportModal" preset="card" title="导出账号" style="width: 400px;">
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
          导出文件经过 AES-256-GCM 加密，请妥善保管密码
        </n-alert>
        <n-button type="primary" block :loading="exportLoading" @click="doExport">
          确认导出
        </n-button>
      </n-form>
    </n-modal>

    <!-- 导入弹窗 -->
    <n-modal v-model:show="showImportModal" preset="card" title="导入账号" style="width: 420px;">
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

    <!-- OAuth 登录弹窗 -->
    <n-modal
      :show="showOAuthModal"
      preset="card"
      title="OAuth 网页登录"
      style="width: 540px;"
      @update:show="handleOAuthModalVisibleChange"
    >
      <n-space vertical size="medium">
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
          <n-space>
            <n-button
              type="primary"
              :disabled="!oauthLogin"
              :loading="oauthOpening"
              @click="handleOpenOAuthLoginUrl"
            >
              打开授权页
            </n-button>
            <n-button :loading="oauthCancelling" @click="handleCancelOAuthLogin">
              取消登录
            </n-button>
          </n-space>
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
import { useAccountStore } from '@/stores/account'
import { accountService, authService } from '@/services'
import { AUTH_TYPE_LABELS, STATUS_LABELS } from '@/types'
import type {
  Account,
  AccountStatus,
  OAuthCallbackFinishedEvent,
  OAuthLoginResult,
  PreparedOAuthLogin,
} from '@/types'
import StatusDot from '@/components/common/StatusDot.vue'
import AccountCard from '@/components/account/AccountCard.vue'
import CreateAccountModal from '@/components/account/CreateAccountModal.vue'

const router = useRouter()
const message = useMessage()
const dialog = useDialog()
const accountStore = useAccountStore()
const { accounts, loading, checkingStatus, totalAccounts, accountsByStatus } =
  storeToRefs(accountStore)

// 本地状态
const searchQuery = ref('')
const filterStatus = ref<AccountStatus | null>(null)
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

// 计算属性
const filteredAccounts = computed(() => {
  let list = accounts.value
  if (filterStatus.value) {
    list = list.filter((a) => a.status === filterStatus.value)
  }
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    list = list.filter(
      (a) =>
        a.name.toLowerCase().includes(q) ||
        a.email?.toLowerCase().includes(q) ||
        a.organization?.toLowerCase().includes(q),
    )
  }
  return list
})

// SVG 图标
const PlusIcon = { render: () => h('svg', { width: 16, height: 16, viewBox: '0 0 24 24', fill: 'none' }, [h('path', { d: 'M12 5v14M5 12h14', stroke: 'currentColor', 'stroke-width': 2, 'stroke-linecap': 'round' })]) }
const RefreshIcon = { render: () => h('svg', { width: 16, height: 16, viewBox: '0 0 24 24', fill: 'none' }, [h('path', { d: 'M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15', stroke: 'currentColor', 'stroke-width': 2, 'stroke-linecap': 'round' })]) }
const SearchIcon = { render: () => h('svg', { width: 14, height: 14, viewBox: '0 0 24 24', fill: 'none' }, [h('circle', { cx: 11, cy: 11, r: 8, stroke: 'currentColor', 'stroke-width': 2 }), h('path', { d: 'm21 21-4.35-4.35', stroke: 'currentColor', 'stroke-width': 2, 'stroke-linecap': 'round' })]) }

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

// 操作
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

async function handleSetDefault(id: string) {
  await accountStore.switchAccount(id)
  message.success('已设为默认账号')
}

function handleDelete(account: Account) {
  dialog.warning({
    title: '删除账号',
    content: `确定要删除账号「${account.name}」吗？此操作不可撤销，相关数据将一并清除。`,
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
  message.success(`账号「${account.name}」创建成功`)
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
      message.warning(`已同步账号「${result.account_name}」，账号信息获取失败`)
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
    // 无文件选择器时使用剪贴板作为导出兜底，避免凭证明文落盘。
    await navigator.clipboard.writeText(encrypted)
    showExportModal.value = false
    message.success('导出成功，已复制到剪贴板')
  } catch (e) {
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
  } catch (e) {
    message.error('导入失败：密码错误或数据损坏')
  } finally {
    importLoading.value = false
  }
}
</script>

<style scoped>
.view-container {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 1100px;
}

.view-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.view-title {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -0.3px;
}

.view-sub {
  font-size: 13px;
  color: var(--text-secondary);
  margin-top: 2px;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.status-bar {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.status-chip {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  border-radius: 20px;
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  cursor: pointer;
  font-size: 12px;
  color: var(--text-secondary);
  transition: all 0.15s;
}
.status-chip:hover { border-color: var(--accent); color: var(--accent); }
.status-chip.active { border-color: var(--accent); background: var(--accent-soft); color: var(--accent); }
.chip-count {
  font-size: 11px;
  background: rgba(255,255,255,0.08);
  border-radius: 10px;
  padding: 1px 6px;
  font-weight: 600;
}

.search-row { display: flex; max-width: 360px; }

.account-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 12px;
}

.loading-state, .empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 80px 0;
  color: var(--text-secondary);
  font-size: 14px;
}
</style>
