<template>
  <div class="app-page">
    <section class="surface-panel section-grid dashboard-panel">
      <div class="toolbar-header">
        <h1 class="panel-heading">仪表盘</h1>
      </div>

      <div class="dashboard-summary">
        <div class="dashboard-summary-row">
          <div class="dashboard-summary-card metric-total-card">
            <span class="metric-label">总账号数</span>
            <strong class="metric-value">{{ totalAccountCount }}</strong>
          </div>
          <div class="dashboard-summary-card metric-available-card">
            <span class="metric-label">可用账号</span>
            <strong class="metric-value">{{ availableAccountCount }}</strong>
          </div>
          <div class="dashboard-summary-card metric-request-card">
            <span class="metric-label">近一年请求</span>
            <strong class="metric-value">{{ (summary?.total_requests ?? 0).toLocaleString() }}</strong>
          </div>
        </div>
        <div class="dashboard-summary-row dashboard-token-summary-row">
          <div class="dashboard-summary-card metric-today-card">
            <span class="metric-label">今日 Token</span>
            <strong class="metric-value">{{ formatTokens(todayTotalTokens) }}</strong>
          </div>
          <div class="dashboard-summary-card metric-input-card">
            <span class="metric-label">近一年输入 Token</span>
            <strong class="metric-value">{{ formatTokens(summary?.total_input_tokens ?? 0) }}</strong>
          </div>
          <div class="dashboard-summary-card metric-cached-card">
            <span class="metric-label">近一年缓存命中</span>
            <strong class="metric-value">{{ formatTokens(summary?.total_cached_input_tokens ?? 0) }}</strong>
          </div>
          <div class="dashboard-summary-card metric-output-card">
            <span class="metric-label">近一年输出 Token</span>
            <strong class="metric-value">{{ formatTokens(summary?.total_output_tokens ?? 0) }}</strong>
          </div>
        </div>
      </div>
    </section>

    <TokenUsageHeatmap :loading="loading" :daily-data="chartData" />

    <section v-if="!loading" class="surface-panel account-detail-panel">
      <div class="account-detail-head">
        <div class="account-detail-title-group">
          <h2 class="panel-heading">账号用量明细</h2>
          <span class="account-detail-period-label">{{ selectedDetailPeriodLabel }}</span>
        </div>

        <div class="account-detail-actions">
          <div class="detail-period-tabs" role="group" aria-label="账号用量明细周期">
            <button
              v-for="option in detailPeriodOptions"
              :key="option.value"
              type="button"
              class="detail-period-tab"
              :class="{ active: detailPeriod === option.value }"
              @click="setDetailPeriod(option.value)"
            >
              {{ option.label }}
            </button>
          </div>
          <span class="account-detail-count">{{ summaryRows.length }} 个账号</span>
        </div>
      </div>

      <div v-if="summaryRows.length > 0" class="account-detail-grid">
        <article
          v-for="(row, index) in summaryRows"
          :key="row.account_id"
          class="account-detail-card"
        >
          <div class="account-detail-card-head">
            <div class="account-detail-card-copy">
              <span class="account-detail-rank">TOP {{ index + 1 }}</span>
              <h3>{{ row.account_name }}</h3>
            </div>
            <div class="account-detail-card-total">
              <span>总 Token</span>
              <strong>{{ formatTokens(row.total_tokens) }}</strong>
            </div>
          </div>

          <div class="account-detail-metric-grid">
            <div class="account-detail-metric">
              <span>输入 Token</span>
              <strong>{{ formatTokens(row.input_tokens) }}</strong>
            </div>
            <div class="account-detail-metric">
              <span>输出 Token</span>
              <strong>{{ formatTokens(row.output_tokens) }}</strong>
            </div>
            <div class="account-detail-metric">
              <span>缓存命中</span>
              <strong>{{ formatTokens(row.cached_input_tokens) }}</strong>
            </div>
            <div class="account-detail-metric">
              <span>请求次数</span>
              <strong>{{ row.request_count.toLocaleString() }}</strong>
            </div>
          </div>
        </article>
      </div>

      <div v-else class="account-detail-empty">
        <p>{{ detailEmptyText }}</p>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useMessage } from 'naive-ui'
import { useAccountStore } from '@/stores/account'
import { useUsageStore } from '@/stores/usage'
import type { ChartDataPoint, UsagePeriod } from '@/types'
import { resolveAccountDisplayName } from '@/utils/account-display'
import TokenUsageHeatmap from '@/components/usage/TokenUsageHeatmap.vue'

interface UsageSummaryRow {
  account_id: string
  account_name: string
  total_tokens: number
  input_tokens: number
  cached_input_tokens: number
  output_tokens: number
  request_count: number
}

interface DetailPeriodOption {
  label: string
  value: UsagePeriod
  emptyText: string
}

const accountStore = useAccountStore()
const usageStore = useUsageStore()
const message = useMessage()
const annualPeriod: UsagePeriod = 'year'
const detailPeriod = ref<UsagePeriod>('day')
const loading = ref(false)
const initialized = ref(false)
const detailPeriodOptions: DetailPeriodOption[] = [
  {
    label: '今年',
    value: 'current_year',
    emptyText: '今年还没有可展示的账号用量数据。',
  },
  {
    label: '本月',
    value: 'current_month',
    emptyText: '本月还没有可展示的账号用量数据。',
  },
  {
    label: '今天',
    value: 'day',
    emptyText: '今天还没有可展示的账号用量数据。',
  },
]

const accountIds = computed(() => accountStore.accounts.map((account) => account.id))
const totalAccountCount = computed(() => accountStore.accounts.length)
const availableAccountCount = computed(
  () =>
    accountStore.accounts.filter(
      (account) => account.is_active && !['error', 'expired'].includes(account.status),
    ).length,
)

const summary = computed(() =>
  usageStore.getSummaryForAccounts(accountIds.value, annualPeriod),
)

const chartData = computed<ChartDataPoint[]>(() =>
  usageStore.getChartDataForAccounts(accountIds.value, annualPeriod),
)

const todayTotalTokens = computed(() => {
  const todayPoint = chartData.value.find((point) => point.date === formatDateKey(new Date()))
  return (todayPoint?.input_tokens ?? 0) + (todayPoint?.output_tokens ?? 0)
})

const selectedDetailPeriodOption = computed(
  () =>
    detailPeriodOptions.find((option) => option.value === detailPeriod.value) ??
    detailPeriodOptions[0],
)
const selectedDetailPeriodLabel = computed(() => selectedDetailPeriodOption.value.label)
const detailEmptyText = computed(() => selectedDetailPeriodOption.value.emptyText)

const summaryRows = computed<UsageSummaryRow[]>(() =>
  accountStore.accounts
    .map((account) => {
      const accountSummary = usageStore.getSummary(account.id, detailPeriod.value)
      if (!accountSummary) {
        return null
      }

      const totalTokens =
        accountSummary.total_input_tokens +
        accountSummary.total_output_tokens
      const activityWeight = totalTokens + accountSummary.total_requests
      if (activityWeight <= 0) {
        return null
      }

      return {
        account_id: account.id,
        account_name: resolveAccountDisplayName(account),
        total_tokens: totalTokens,
        input_tokens: accountSummary.total_input_tokens,
        cached_input_tokens: accountSummary.total_cached_input_tokens,
        output_tokens: accountSummary.total_output_tokens,
        request_count: accountSummary.total_requests,
      }
    })
    .filter((row): row is UsageSummaryRow => Boolean(row))
    .sort((left, right) => {
      const leftWeight = left.total_tokens + left.request_count
      const rightWeight = right.total_tokens + right.request_count
      return rightWeight - leftWeight
    }),
)

function formatTokens(value: number): string {
  const units = [
    { threshold: 1_000_000_000_000, label: 'T' },
    { threshold: 1_000_000_000, label: 'B' },
    { threshold: 1_000_000, label: 'M' },
    { threshold: 1_000, label: 'K' },
  ]

  const compactUnit = units.find((unit) => value >= unit.threshold)
  if (!compactUnit) {
    return Math.round(value).toLocaleString()
  }

  return `${(value / compactUnit.threshold).toFixed(1)} ${compactUnit.label}`
}

async function loadData() {
  const currentAccountIds = accountIds.value
  if (currentAccountIds.length === 0) {
    return
  }

  const selectedPeriod = detailPeriod.value
  const hasAnnualCache = usageStore.hasCachedUsageForAccounts(currentAccountIds, annualPeriod)
  const hasDetailCache = usageStore.hasCachedUsageForAccounts(currentAccountIds, selectedPeriod)
  loading.value = !(hasAnnualCache && hasDetailCache)
  try {
    if (!hasAnnualCache) {
      await usageStore.loadCachedUsageForAccounts(currentAccountIds, annualPeriod)
    }

    if (!hasDetailCache && selectedPeriod !== annualPeriod) {
      await usageStore.loadCachedUsageForAccounts(currentAccountIds, selectedPeriod)
    }

    if (
      usageStore.hasCachedUsageForAccounts(currentAccountIds, annualPeriod) &&
      usageStore.hasCachedUsageForAccounts(currentAccountIds, selectedPeriod)
    ) {
      loading.value = false
    }

    await usageStore.refreshUsageForAccounts(currentAccountIds, annualPeriod)
    if (selectedPeriod !== annualPeriod) {
      await usageStore.loadCachedUsageForAccounts(currentAccountIds, selectedPeriod)
    }
  } catch (error) {
    console.warn('刷新 Token 用量失败', error)
    message.error('刷新 Token 用量失败')
  } finally {
    loading.value = false
  }
}

function setDetailPeriod(period: UsagePeriod) {
  if (detailPeriod.value === period) {
    return
  }

  detailPeriod.value = period
}

function formatDateKey(date: Date): string {
  const year = date.getFullYear()
  const month = `${date.getMonth() + 1}`.padStart(2, '0')
  const day = `${date.getDate()}`.padStart(2, '0')
  return `${year}-${month}-${day}`
}

watch(
  () => accountIds.value.join('|'),
  (currentValue, previousValue) => {
    if (!initialized.value || !currentValue || currentValue === previousValue) {
      return
    }

    void loadData()
  },
)

watch(detailPeriod, () => {
  if (!initialized.value || accountIds.value.length === 0) {
    return
  }

  void loadData()
})

onMounted(async () => {
  if (accountStore.accounts.length === 0) {
    await accountStore.loadAccounts()
  }

  if (accountIds.value.length > 0) {
    await loadData()
  }

  initialized.value = true
})
</script>

<style scoped>
.toolbar-header {
  display: flex;
  align-items: center;
  gap: 12px;
}

.dashboard-panel {
  gap: 18px;
}

.dashboard-summary {
  display: grid;
  gap: 12px;
}

.dashboard-summary-row {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}

.dashboard-token-summary-row {
  grid-template-columns: repeat(4, minmax(0, 1fr));
}

.dashboard-summary-card {
  --card-accent: var(--app-blue);
  --card-border: rgba(0, 113, 227, 0.14);
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
  padding: 16px;
  border-radius: 22px;
  border: 1px solid var(--card-border);
  background: transparent;
}

.dashboard-summary-card .metric-label {
  color: var(--card-accent);
}

.metric-total-card {
  --card-accent: #4f46e5;
  --card-border: rgba(79, 70, 229, 0.16);
}

.metric-available-card {
  --card-accent: #248a3d;
  --card-border: rgba(52, 199, 89, 0.18);
}

.metric-today-card {
  --card-accent: #b7791f;
  --card-border: rgba(245, 158, 11, 0.18);
}

.metric-input-card {
  --card-accent: #0071e3;
  --card-border: rgba(0, 113, 227, 0.18);
}

.metric-output-card {
  --card-accent: #06b6d4;
  --card-border: rgba(6, 182, 212, 0.2);
}

.metric-cached-card {
  --card-accent: #c2410c;
  --card-border: rgba(194, 65, 12, 0.18);
}

.metric-request-card {
  --card-accent: #7c3aed;
  --card-border: rgba(124, 58, 237, 0.16);
}

.account-detail-panel {
  display: grid;
  gap: 14px;
}

.account-detail-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.account-detail-title-group {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  min-width: 0;
}

.account-detail-period-label {
  display: inline-flex;
  align-items: center;
  min-height: 24px;
  padding: 0 10px;
  border-radius: var(--app-radius-control);
  background: rgba(0, 113, 227, 0.1);
  color: var(--app-blue);
  font-size: 11px;
  line-height: 1.33;
}

.account-detail-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  flex-wrap: wrap;
}

.detail-period-tabs {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px;
  border-radius: var(--app-radius-control);
  background: var(--app-surface-muted);
}

.detail-period-tab {
  border: none;
  border-radius: calc(var(--app-radius-control) - 4px);
  background: transparent;
  color: var(--app-ink-secondary);
  min-height: 28px;
  padding: 0 11px;
  font-size: 12px;
  line-height: 1.33;
  cursor: pointer;
  transition:
    background-color 0.18s ease,
    color 0.18s ease;
}

.detail-period-tab.active {
  background: var(--app-surface);
  color: var(--app-ink);
  box-shadow: 0 1px 4px rgba(29, 29, 31, 0.08);
}

.account-detail-count {
  display: inline-flex;
  align-items: center;
  min-height: 28px;
  padding: 0 12px;
  border-radius: var(--app-radius-control);
  background: var(--app-surface-muted);
  color: var(--app-ink-secondary);
  font-size: 12px;
  line-height: 1.33;
}

.account-detail-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 10px;
}

.account-detail-card {
  display: grid;
  gap: 10px;
  padding: 12px;
  border-radius: 18px;
  border: 1px solid rgba(29, 29, 31, 0.08);
  background: var(--app-surface-muted);
}

.account-detail-card-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.account-detail-card-copy {
  display: grid;
  gap: 4px;
  min-width: 0;
}

.account-detail-rank {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-blue);
}

.account-detail-card-copy h3,
.account-detail-card-total strong {
  margin: 0;
  font-family: var(--font-display);
}

.account-detail-card-copy h3 {
  font-size: 15px;
  line-height: 1.28;
  color: var(--app-ink);
}

.account-detail-card-total {
  display: grid;
  gap: 4px;
  flex-shrink: 0;
  text-align: right;
}

.account-detail-card-total span,
.account-detail-metric span {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
}

.account-detail-card-total strong {
  font-size: 18px;
  line-height: 1.2;
  color: var(--app-ink);
}

.account-detail-metric-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 6px;
}

.account-detail-metric {
  display: grid;
  gap: 4px;
  padding: 8px;
  border-radius: 14px;
  background: var(--app-surface);
}

.account-detail-metric strong {
  font-family: var(--font-display);
  font-size: 14px;
  line-height: 1.24;
  color: var(--app-ink);
}

.account-detail-empty {
  min-height: 120px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--app-ink-secondary);
}

.account-detail-empty p {
  margin: 0;
}

@media (max-width: 960px) {
  .dashboard-summary-row {
    grid-template-columns: 1fr;
  }

  .account-detail-head,
  .account-detail-card-head {
    flex-direction: column;
    align-items: stretch;
  }

  .account-detail-actions {
    justify-content: flex-start;
  }

  .account-detail-metric-grid {
    grid-template-columns: 1fr;
  }

  .account-detail-card-total {
    text-align: left;
  }
}
</style>
