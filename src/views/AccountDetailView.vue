<template>
  <div v-if="account" class="app-page">
    <n-breadcrumb class="page-breadcrumb">
      <n-breadcrumb-item @click="router.push('/accounts')" style="cursor: pointer;">
        账号列表
      </n-breadcrumb-item>
      <n-breadcrumb-item>{{ displayName }}</n-breadcrumb-item>
    </n-breadcrumb>

    <section class="page-hero page-hero-light">
      <div class="page-hero-copy">
        <h1 class="page-title">{{ displayName }}</h1>
        <p class="page-subtitle">
          {{ AUTH_TYPE_LABELS[account.auth_type] }}
          <template v-if="displaySubtitleEmail"> · {{ displaySubtitleEmail }}</template>
          <template v-if="displayOrganization"> · {{ displayOrganization }}</template>
        </p>
        <div class="page-hero-actions">
          <n-button secondary :loading="checking" @click="handleCheck">检测状态</n-button>
          <n-button
            secondary
            :disabled="account.is_default"
            @click="handleSwitchAccount"
          >
            切换账号
          </n-button>
          <n-button secondary @click="showEditModal = true">编辑账号</n-button>
          <n-button secondary :loading="refreshing" @click="handleRefreshToken">
            刷新 Token
          </n-button>
        </div>
      </div>

      <div class="hero-detail-panel">
        <div class="hero-avatar" :style="{ background: account.color }">
          {{ displayAvatarText }}
        </div>
        <div class="hero-status">
          <StatusDot
            :status="statusDisplay.tone"
            :label="statusDisplay.label"
            :title="statusDisplay.title"
            show-label
          />
        </div>
        <div class="hero-pill-list">
          <span v-if="account.is_default" class="hero-pill hero-pill-dark">默认账号</span>
          <span
            v-if="account.codex_plan_type"
            class="hero-pill"
            :class="`hero-pill-plan-${planTone}`"
          >
            {{ planLabel }}
          </span>
          <span class="hero-pill">创建于 {{ formatDate(account.created_at) }}</span>
        </div>
        <div v-if="hasCodexUsage" class="detail-quota-grid">
          <div v-if="showFiveHourQuota" class="detail-quota-item">
            <span>5 小时剩余</span>
            <strong>{{ formatRemainingUsageWindow(account.codex_usage_5h) }}</strong>
          </div>
          <div class="detail-quota-item">
            <span>7 天剩余</span>
            <strong>{{ formatRemainingUsageWindow(account.codex_usage_week) }}</strong>
          </div>
          <div v-if="nextUsageResetAt" class="detail-quota-reset">
            下次重置 {{ formatUnixDate(nextUsageResetAt) }}
          </div>
        </div>
        <div v-if="displayStatusMessage" class="status-message" :class="statusDisplay.tone">
          {{ displayStatusMessage }}
        </div>
        <div v-if="statusDiagnostic" class="status-diagnostic">
          {{ statusDiagnostic }}
        </div>
      </div>
    </section>

    <section class="two-column-grid">
      <div class="surface-panel">
        <h2 class="panel-heading">账号信息</h2>
        <p class="panel-copy">基础信息。</p>
        <div class="data-pair-list detail-list">
          <div class="data-pair">
            <span class="data-pair-label">账号 ID</span>
            <span class="data-pair-value account-id">{{ account.id }}</span>
          </div>
          <div class="data-pair">
            <span class="data-pair-label">认证方式</span>
            <span class="data-pair-value">{{ AUTH_TYPE_LABELS[account.auth_type] }}</span>
          </div>
          <div class="data-pair">
            <span class="data-pair-label">状态</span>
            <span class="data-pair-value">
              <StatusDot
                :status="statusDisplay.tone"
                :label="statusDisplay.label"
                :title="statusDisplay.title"
                show-label
              />
            </span>
          </div>
          <div class="data-pair">
            <span class="data-pair-label">最后更新</span>
            <span class="data-pair-value">{{ formatDate(account.updated_at) }}</span>
          </div>
          <div class="data-pair">
            <span class="data-pair-label">最后检测</span>
            <span class="data-pair-value">
              {{ account.last_checked_at ? formatDate(account.last_checked_at) : '从未检测' }}
            </span>
          </div>
        </div>
      </div>

      <div class="surface-panel surface-panel-dark">
        <h2 class="panel-heading">用量快览</h2>
        <p class="panel-copy">本月汇总。</p>
        <div v-if="usageLoading" class="panel-loading">
          <n-spin size="small" />
        </div>
        <div v-else-if="summary" class="metric-grid usage-metrics">
          <div class="metric-card">
            <span class="metric-label">输入 Token</span>
            <strong class="metric-value">{{ formatTokens(summary.total_input_tokens) }}</strong>
          </div>
          <div class="metric-card">
            <span class="metric-label">输出 Token</span>
            <strong class="metric-value">{{ formatTokens(summary.total_output_tokens) }}</strong>
          </div>
          <div class="metric-card">
            <span class="metric-label">请求次数</span>
            <strong class="metric-value">{{ summary.total_requests }}</strong>
          </div>
          <div class="metric-card">
            <span class="metric-label">费用估算</span>
            <strong class="metric-value">${{ summary.total_cost.toFixed(4) }}</strong>
          </div>
        </div>
        <div v-else class="usage-empty">
          <p>当前还没有可展示的用量统计。</p>
        </div>
        <button class="detail-link" type="button" @click="goToUsage">查看详细统计</button>
      </div>
    </section>

    <section class="surface-panel">
      <h2 class="panel-heading">认证管理</h2>
      <p class="panel-copy">查看凭证状态与刷新结果。</p>
      <div class="auth-grid">
        <div class="auth-credential-card">
          <div class="auth-credential-head">
            <span class="auth-label">凭证遮罩</span>
            <button
              type="button"
              class="credential-visibility-toggle"
              :title="credentialVisible ? '隐藏凭证' : '显示凭证'"
              @click="handleToggleCredentialVisibility"
            >
              <svg
                v-if="credentialVisible"
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
              >
                <path
                  d="M3 3l18 18M10.58 10.58A2 2 0 0 0 12 14a2 2 0 0 0 1.42-.58M9.88 5.09A10.94 10.94 0 0 1 12 5c5 0 9.27 3.11 11 7-0.56 1.26-1.42 2.42-2.51 3.41M6.61 6.61C4.62 7.84 3.08 9.71 2 12c1.73 3.89 6 7 10 7 1.58 0 3.1-.35 4.47-.98"
                  stroke="currentColor"
                  stroke-width="1.6"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
              <svg
                v-else
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
              >
                <path
                  d="M2 12c1.73-3.89 6-7 10-7s8.27 3.11 10 7c-1.73 3.89-6 7-10 7S3.73 15.89 2 12z"
                  stroke="currentColor"
                  stroke-width="1.6"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
                <circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="1.6" />
              </svg>
            </button>
          </div>
          <strong class="auth-value auth-value-mono">{{ credentialDisplayValue }}</strong>
          <span class="auth-note">
            {{ credentialVisible ? '仅在当前窗口临时显示。' : '点击眼睛图标显示凭证。' }}
          </span>
        </div>
        <div class="auth-credential-card auth-action-card">
          <span class="auth-label">刷新操作</span>
          <strong class="auth-action-title">刷新 Token</strong>
          <span class="auth-note">用于校验当前登录状态并更新结果。</span>
          <n-button type="primary" class="refresh-action-button" :loading="refreshing" @click="handleRefreshToken">
            刷新 Token
          </n-button>
        </div>
      </div>
      <n-alert
        v-if="authResult"
        :type="authAlertType"
        :title="authResult.message"
        style="margin-top: 18px;"
      />
    </section>

    <n-modal v-model:show="showEditModal" preset="card" title="编辑账号" style="width: 520px;">
      <n-form :model="editForm" label-placement="top">
        <n-form-item label="邮箱">
          <n-input v-model:value="editForm.email" />
        </n-form-item>
        <n-form-item label="组织">
          <n-input v-model:value="editForm.organization" />
        </n-form-item>
        <n-form-item label="更新凭证（留空保持不变）">
          <n-input
            v-model:value="editForm.credential_value"
            type="password"
            show-password-on="click"
            placeholder="留空则不修改凭证"
          />
        </n-form-item>
        <n-form-item label="标识颜色">
          <div class="color-row">
            <button
              v-for="color in PRESET_COLORS"
              :key="color"
              type="button"
              class="color-dot"
              :class="{ selected: editForm.color === color }"
              :style="{ background: color }"
              @click="editForm.color = color"
            />
          </div>
        </n-form-item>
        <div class="modal-footer">
          <n-button secondary @click="showEditModal = false">取消</n-button>
          <n-button type="primary" :loading="editLoading" @click="handleEdit">保存</n-button>
        </div>
      </n-form>
    </n-modal>
  </div>

  <div v-else class="app-page">
    <section class="surface-panel empty-panel">
      <n-spin />
      <p>正在加载账号详情。</p>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { useAccountStore } from '@/stores/account'
import { useUsageStore } from '@/stores/usage'
import { accountService, authService } from '@/services'
import { AUTH_TYPE_LABELS } from '@/types'
import type { AuthCheckResult, CodexUsageWindow } from '@/types'
import StatusDot from '@/components/common/StatusDot.vue'
import { format, parseISO } from 'date-fns'
import {
  resolveAccountAvatarText,
  resolveAccountDisplayName,
  resolveAccountOrganizationDisplay,
} from '@/utils/account-display'
import {
  formatAccountPlanType,
  resolveAccountPlanTone,
  supportsFiveHourQuota,
} from '@/utils/account-plan'
import {
  resolveAccountStatusDiagnostic,
  resolveAccountStatusDisplay,
  resolveAccountStatusMessage,
} from '@/utils/account-status'

const route = useRoute()
const router = useRouter()
const message = useMessage()
const accountStore = useAccountStore()
const usageStore = useUsageStore()

const accountId = computed(() => route.params.id as string)
const account = computed(() => accountStore.accounts.find((item) => item.id === accountId.value))
const displayName = computed(() => (account.value ? resolveAccountDisplayName(account.value) : ''))
const displaySubtitleEmail = computed(() => {
  const email = account.value?.email?.trim() ?? ''
  return email && email !== displayName.value ? email : ''
})
const displayOrganization = computed(() =>
  account.value ? resolveAccountOrganizationDisplay(account.value) : null,
)
const displayAvatarText = computed(() =>
  account.value ? resolveAccountAvatarText(account.value) : '?',
)
const statusDisplay = computed(() => resolveAccountStatusDisplay(account.value))
const displayStatusMessage = computed(() => resolveAccountStatusMessage(account.value))
const statusDiagnostic = computed(() => resolveAccountStatusDiagnostic(account.value))
const planLabel = computed(() => formatAccountPlanType(account.value?.codex_plan_type))
const planTone = computed(() => resolveAccountPlanTone(account.value?.codex_plan_type))
const showFiveHourQuota = computed(() => supportsFiveHourQuota(account.value?.codex_plan_type))

const checking = computed(() => accountStore.checkingStatus.has(accountId.value))
const usageLoading = ref(false)
const summary = computed(() => usageStore.getSummary(accountId.value, 'month'))
const hasCodexUsage = computed(() =>
  Boolean(account.value?.codex_usage_5h || account.value?.codex_usage_week),
)
const nextUsageResetAt = computed(() =>
  account.value ? resolveNextUsageResetAt(account.value.codex_usage_5h, account.value.codex_usage_week) : null,
)

const showEditModal = ref(false)
const editLoading = ref(false)
const refreshing = ref(false)
const authResult = ref<AuthCheckResult | null>(null)
const credentialVisible = ref(false)
const credentialLoading = ref(false)
const credentialPreview = ref('')

const authAlertType = computed(() => {
  if (!authResult.value) return 'default'
  return {
    valid: 'success',
    expired: 'warning',
    invalid: 'error',
    unknown: 'default',
  }[authResult.value.status] as 'success' | 'warning' | 'error' | 'default'
})

const credentialDisplayValue = computed(() => {
  if (credentialLoading.value) {
    return '正在读取...'
  }

  if (!credentialVisible.value) {
    return '•'.repeat(32)
  }

  return credentialPreview.value || '未读取到凭证'
})

const PRESET_COLORS = [
  '#0071e3',
  '#1f8f5f',
  '#b26a00',
  '#c4314b',
  '#7254d1',
  '#0f9fb0',
  '#d96a20',
  '#b53b70',
  '#147d68',
  '#64748b',
]

const editForm = ref({
  name: '',
  email: '',
  organization: '',
  color: '#0071e3',
  credential_value: '',
})

onMounted(async () => {
  if (account.value) {
    editForm.value = {
      name: account.value.name,
      email: account.value.email ?? '',
      organization: account.value.organization ?? '',
      color: account.value.color,
      credential_value: '',
    }
  }

  usageLoading.value = true
  try {
    await usageStore.loadUsage(accountId.value, 'month')
  } finally {
    usageLoading.value = false
  }
})

function formatDate(iso: string) {
  try {
    return format(parseISO(iso), 'yyyy-MM-dd HH:mm')
  } catch {
    return iso
  }
}

function formatUnixDate(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return '未知'
  return format(new Date(seconds * 1000), 'yyyy-MM-dd HH:mm')
}

function formatTokens(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`
  return String(value)
}

function formatRemainingUsageWindow(window?: CodexUsageWindow): string {
  if (!window) return '未知'
  return formatPercent(100 - window.used_percent)
}

function formatPercent(value: number): string {
  if (!Number.isFinite(value)) return '未知'
  return `${Math.max(0, Math.min(100, value)).toFixed(1)}%`
}

function resolveNextUsageResetAt(
  fiveHour?: CodexUsageWindow,
  oneWeek?: CodexUsageWindow,
): number | null {
  const resetTimes = [fiveHour?.reset_at, oneWeek?.reset_at]
    .filter((value): value is number => typeof value === 'number' && Number.isFinite(value) && value > 0)

  return resetTimes.length > 0 ? Math.min(...resetTimes) : null
}

async function handleCheck() {
  await accountStore.checkAccountStatus(accountId.value)
  message.success('状态检测完成')
}

async function handleSwitchAccount() {
  try {
    await accountStore.switchAccount(accountId.value)
    message.success('已切换当前账号')
  } catch (error) {
    message.error(getErrorMessage(error, '切换账号失败'))
  }
}

async function handleRefreshToken() {
  refreshing.value = true
  try {
    authResult.value = await authService.refreshToken(accountId.value)
  } catch {
    message.error('刷新失败')
  } finally {
    refreshing.value = false
  }
}

async function handleToggleCredentialVisibility() {
  if (credentialVisible.value) {
    credentialVisible.value = false
    return
  }

  if (credentialPreview.value) {
    credentialVisible.value = true
    return
  }

  credentialLoading.value = true
  try {
    const credential = await accountService.getAccountCredential(accountId.value)
    credentialPreview.value = extractCredentialPreview(credential)
    credentialVisible.value = true
  } catch {
    message.error('读取凭证失败')
  } finally {
    credentialLoading.value = false
  }
}

function extractCredentialPreview(credential: string): string {
  const trimmedCredential = credential.trim()
  if (!trimmedCredential) {
    return ''
  }

  if (!trimmedCredential.startsWith('{')) {
    return trimmedCredential
  }

  try {
    const parsedCredential = JSON.parse(trimmedCredential)
    const accessToken = parsedCredential?.tokens?.access_token
    if (typeof accessToken === 'string' && accessToken.trim()) {
      return accessToken.trim()
    }

    const apiKey = parsedCredential?.OPENAI_API_KEY
    if (typeof apiKey === 'string' && apiKey.trim()) {
      return apiKey.trim()
    }
  } catch {
    return trimmedCredential
  }

  return trimmedCredential
}

function getErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message) return error.message
  if (typeof error === 'string' && error) return error
  return fallback
}

async function handleEdit() {
  editLoading.value = true
  try {
    const normalizedEmail = editForm.value.email.trim()
    await accountStore.updateAccount({
      id: accountId.value,
      name: normalizedEmail || editForm.value.name,
      email: normalizedEmail || undefined,
      organization: editForm.value.organization || undefined,
      color: editForm.value.color,
      credential_value: editForm.value.credential_value || undefined,
    })
    showEditModal.value = false
    message.success('账号已更新')
  } finally {
    editLoading.value = false
  }
}

function goToUsage() {
  accountStore.setActive(accountId.value)
  router.push('/usage')
}
</script>

<style scoped>
.page-breadcrumb {
  margin-bottom: -8px;
}

.hero-detail-panel {
  width: min(280px, 100%);
  display: flex;
  flex-direction: column;
  gap: 10px;
  align-items: flex-start;
}

.hero-avatar {
  width: 56px;
  height: 56px;
  border-radius: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-display);
  font-size: 22px;
  font-weight: 600;
  color: #ffffff;
}

.hero-status {
  display: flex;
  align-items: center;
}

.hero-pill-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.hero-pill {
  display: inline-flex;
  align-items: center;
  min-height: 28px;
  padding: 0 12px;
  border-radius: var(--app-radius-control);
  background: var(--app-surface-muted);
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-secondary);
}

.hero-pill-dark {
  background: var(--app-ink);
  color: #ffffff;
}

.hero-pill-plan-green {
  background: rgba(52, 199, 89, 0.14);
  color: #248a3d;
}

.hero-pill-plan-blue {
  background: rgba(0, 113, 227, 0.12);
  color: var(--app-blue);
}

.hero-pill-plan-purple {
  background: rgba(139, 92, 246, 0.14);
  color: #6e44d9;
}

.hero-pill-plan-neutral {
  background: rgba(29, 29, 31, 0.08);
  color: var(--app-ink);
}

.detail-quota-grid {
  width: 100%;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.detail-quota-item {
  padding: 10px 12px;
  border-radius: 16px;
  background: var(--app-surface-muted);
}

.detail-quota-item span,
.detail-quota-reset {
  display: block;
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
}

.detail-quota-item strong {
  display: block;
  margin-top: 4px;
  font-family: var(--font-display);
  font-size: 16px;
  line-height: 1.2;
}

.detail-quota-reset {
  grid-column: 1 / -1;
}

.detail-list .data-pair-value {
  display: flex;
  justify-content: flex-end;
}

.status-diagnostic {
  font-size: 12px;
  line-height: 1.5;
  color: var(--app-ink-tertiary);
  word-break: break-word;
}

.account-id {
  word-break: break-all;
}

.panel-loading,
.usage-empty {
  min-height: 120px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.usage-empty p {
  margin: 0;
  color: var(--app-feature-ink-secondary);
}

.usage-metrics {
  margin-top: 14px;
}

.usage-metrics :deep(.metric-card) {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.usage-metrics :deep(.metric-value) {
  margin-top: 0;
}

.detail-link {
  margin-top: 12px;
  border: none;
  background: transparent;
  color: var(--app-link-dark);
  padding: 0;
  font-size: 13px;
  line-height: 1.43;
  letter-spacing: -0.12px;
  cursor: pointer;
}

.auth-grid {
  margin-top: 14px;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.auth-credential-card {
  padding: 14px 16px;
  border-radius: 18px;
  background: var(--app-surface-muted);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.auth-credential-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.credential-visibility-toggle {
  width: 30px;
  height: 30px;
  border: none;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 113, 227, 0.08);
  color: var(--app-blue);
  cursor: pointer;
  transition:
    background-color 0.18s ease,
    transform 0.18s ease;
}

.credential-visibility-toggle:hover {
  background: rgba(0, 113, 227, 0.14);
  transform: translateY(-1px);
}

.auth-label {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
}

.auth-value {
  font-family: var(--font-display);
  font-size: 16px;
  line-height: 1.2;
  letter-spacing: 0.12px;
}

.auth-value-mono {
  font-family: ui-monospace, SFMono-Regular, "SFMono-Regular", Consolas, monospace;
  font-size: 13px;
  line-height: 1.6;
  letter-spacing: 0;
  word-break: break-all;
}

.auth-note {
  font-size: 12px;
  line-height: 1.5;
  color: var(--app-ink-secondary);
}

.auth-action-card {
  justify-content: space-between;
}

.auth-action-title {
  font-family: var(--font-display);
  font-size: 18px;
  line-height: 1.2;
  letter-spacing: 0.12px;
}

.refresh-action-button {
  width: 100%;
}

.color-row {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.color-dot {
  width: 28px;
  height: 28px;
  border: 2px solid transparent;
  border-radius: 50%;
  cursor: pointer;
  transition:
    transform 0.18s ease,
    box-shadow 0.18s ease;
}

.color-dot:hover {
  transform: scale(1.08);
}

.color-dot.selected {
  border-color: #ffffff;
  box-shadow: 0 0 0 2px rgba(29, 29, 31, 0.16);
}

.modal-footer {
  margin-top: 14px;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

@media (max-width: 768px) {
  .auth-grid {
    grid-template-columns: 1fr;
  }
}
</style>
