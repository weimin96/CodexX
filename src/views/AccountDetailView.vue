<template>
  <div v-if="account" class="app-page">
    <n-breadcrumb class="page-breadcrumb">
      <n-breadcrumb-item @click="router.push('/accounts')" style="cursor: pointer;">
        账号列表
      </n-breadcrumb-item>
      <n-breadcrumb-item>{{ displayName }}</n-breadcrumb-item>
    </n-breadcrumb>

    <section class="page-hero page-hero-light account-detail-hero">
      <div class="account-hero-identity">
        <div class="hero-avatar" :style="{ background: account.color }">
          {{ displayAvatarText }}
        </div>
        <div class="page-hero-copy">
          <div class="hero-title-row">
            <h1 class="page-title">{{ displayName }}</h1>
            <StatusDot
              :status="statusDisplay.tone"
              :label="statusDisplay.label"
              :title="statusDisplay.title"
              show-label
            />
          </div>
          <p class="page-subtitle">
            {{ AUTH_TYPE_LABELS[account.auth_type] }}
            <template v-if="displaySubtitleEmail"> · {{ displaySubtitleEmail }}</template>
            <template v-if="displayOrganization"> · {{ displayOrganization }}</template>
          </p>
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
          <div v-if="displayStatusMessage" class="status-message" :class="statusDisplay.tone">
            {{ displayStatusMessage }}
          </div>
          <div v-if="statusDiagnostic" class="status-diagnostic">
            {{ statusDiagnostic }}
          </div>
        </div>
      </div>

      <div class="quota-summary-card">
        <div class="quota-summary-head">
          <span>Codex 额度</span>
          <strong>{{ planLabel }}</strong>
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
        <p v-else class="quota-empty">暂无额度窗口数据。</p>
      </div>
    </section>

    <section class="detail-content-grid">
      <div class="surface-panel usage-panel">
        <div class="detail-panel-head">
          <div>
            <h2 class="panel-heading">本月用量</h2>
          </div>
          <button class="detail-link" type="button" @click="goToUsage">查看统计</button>
        </div>
        <div v-if="usageLoading" class="panel-loading">
          <n-spin size="small" />
        </div>
        <div v-else-if="summary" class="detail-metric-grid">
          <div class="metric-card metric-card-input">
            <span class="metric-label">输入 Token</span>
            <strong class="metric-value">{{ formatTokens(summary.total_input_tokens) }}</strong>
          </div>
          <div class="metric-card metric-card-output">
            <span class="metric-label">输出 Token</span>
            <strong class="metric-value">{{ formatTokens(summary.total_output_tokens) }}</strong>
          </div>
          <div class="metric-card metric-card-request">
            <span class="metric-label">请求次数</span>
            <strong class="metric-value">{{ summary.total_requests }}</strong>
          </div>
        </div>
        <div v-else class="usage-empty">
          <p>当前还没有可展示的用量统计。</p>
        </div>
      </div>

      <div class="surface-panel account-info-panel">
        <div class="detail-panel-head">
          <div>
            <h2 class="panel-heading">账号信息</h2>
          </div>
        </div>
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
    </section>
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
import { useAccountStore } from '@/stores/account'
import { useUsageStore } from '@/stores/usage'
import { AUTH_TYPE_LABELS } from '@/types'
import type { CodexUsageWindow } from '@/types'
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

const usageLoading = ref(false)
const summary = computed(() => usageStore.getSummary(accountId.value, 'month'))
const hasCodexUsage = computed(() =>
  Boolean(account.value?.codex_usage_5h || account.value?.codex_usage_week),
)
const nextUsageResetAt = computed(() =>
  account.value ? resolveNextUsageResetAt(account.value.codex_usage_5h, account.value.codex_usage_week) : null,
)

onMounted(async () => {
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

function goToUsage() {
  accountStore.setActive(accountId.value)
  router.push('/usage')
}
</script>

<style scoped>
.page-breadcrumb {
  margin-bottom: -8px;
}

.account-detail-hero {
  align-items: stretch;
}

.account-hero-identity {
  display: flex;
  align-items: flex-start;
  gap: 16px;
  min-width: 0;
  flex: 1;
}

.hero-avatar {
  width: 64px;
  height: 64px;
  border-radius: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-display);
  flex-shrink: 0;
  font-size: 24px;
  font-weight: 600;
  color: #ffffff;
}

.hero-title-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
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

.quota-summary-card {
  width: min(320px, 100%);
  display: grid;
  align-content: start;
  gap: 12px;
  padding: 14px;
  border-radius: 20px;
  background:
    linear-gradient(135deg, rgba(0, 113, 227, 0.08), rgba(0, 113, 227, 0.02) 58%),
    var(--app-surface-muted);
  border: 1px solid rgba(0, 113, 227, 0.1);
}

.quota-summary-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.quota-summary-head span {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
}

.quota-summary-head strong {
  font-family: var(--font-display);
  font-size: 14px;
  line-height: 1.3;
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

.quota-empty {
  margin: 0;
  min-height: 68px;
  display: flex;
  align-items: center;
  color: var(--app-ink-secondary);
  font-size: 13px;
  line-height: 1.5;
}

.status-diagnostic {
  font-size: 12px;
  line-height: 1.5;
  color: var(--app-ink-tertiary);
  word-break: break-word;
}

.detail-content-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.2fr) minmax(320px, 0.8fr);
  gap: 16px;
  align-items: stretch;
}

.usage-panel,
.account-info-panel {
  min-width: 0;
}

.detail-panel-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 14px;
}

.detail-metric-grid {
  margin-top: 16px;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}

.detail-metric-grid .metric-card {
  position: relative;
  overflow: hidden;
  min-height: 118px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  border: 1px solid var(--app-border);
  background: var(--app-surface-muted);
}

.detail-metric-grid .metric-card::before {
  content: '';
  position: absolute;
  inset: 0 0 auto;
  height: 3px;
  background: var(--metric-accent, var(--app-blue));
}

.metric-card-input {
  --metric-accent: #0071e3;
}

.metric-card-output {
  --metric-accent: #0f9fb0;
}

.metric-card-request {
  --metric-accent: var(--app-ink);
}

.detail-metric-grid .metric-value {
  margin-top: 12px;
  font-size: clamp(24px, 4vw, 36px);
  letter-spacing: -0.4px;
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
  color: var(--app-ink-secondary);
}

.detail-link {
  border: none;
  border-radius: var(--app-radius-control);
  background: rgba(0, 113, 227, 0.1);
  color: var(--app-blue);
  padding: 7px 12px;
  font-size: 13px;
  line-height: 1.43;
  letter-spacing: -0.12px;
  cursor: pointer;
  transition:
    background-color 0.18s ease,
    transform 0.18s ease;
}

.detail-link:hover {
  background: rgba(0, 113, 227, 0.16);
  transform: translateY(-1px);
}

.detail-list {
  margin-top: 16px;
}

.detail-list .data-pair-value {
  display: flex;
  justify-content: flex-end;
}

.account-id {
  word-break: break-all;
}

@media (max-width: 1024px) {
  .account-detail-hero,
  .account-hero-identity,
  .detail-panel-head {
    flex-direction: column;
  }

  .quota-summary-card,
  .detail-link {
    width: 100%;
  }

  .detail-content-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 768px) {
  .detail-metric-grid,
  .detail-quota-grid {
    grid-template-columns: 1fr;
  }

  .hero-avatar {
    width: 56px;
    height: 56px;
    font-size: 22px;
  }
}
</style>
