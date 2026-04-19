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
          <div class="dashboard-summary-card metric-today-card">
            <span class="metric-label">今日 Token</span>
            <strong class="metric-value">{{ formatTokens(todayTotalTokens) }}</strong>
          </div>
        </div>
        <div class="dashboard-summary-row">
          <div class="dashboard-summary-card metric-input-card">
            <span class="metric-label">输入 Token</span>
            <strong class="metric-value">{{ formatTokens(summary?.total_input_tokens ?? 0) }}</strong>
          </div>
          <div class="dashboard-summary-card metric-output-card">
            <span class="metric-label">输出 Token</span>
            <strong class="metric-value">{{ formatTokens(summary?.total_output_tokens ?? 0) }}</strong>
          </div>
          <div class="dashboard-summary-card metric-request-card">
            <span class="metric-label">请求次数</span>
            <strong class="metric-value">{{ (summary?.total_requests ?? 0).toLocaleString() }}</strong>
          </div>
        </div>
      </div>
    </section>

    <section class="surface-panel section-grid trend-panel">
      <div class="dashboard-chart-head">
        <h2 class="panel-heading dashboard-chart-title">Token 用量趋势</h2>
        <div class="dashboard-chart-controls">
          <div class="control-block">
            <span class="control-label">时间范围</span>
            <n-radio-group v-model:value="selectedPeriod" @update:value="onPeriodChange">
              <n-radio-button value="day">今日</n-radio-button>
              <n-radio-button value="week">本周</n-radio-button>
              <n-radio-button value="month">本月</n-radio-button>
            </n-radio-group>
          </div>

          <div class="control-block">
            <span class="control-label">图表类型</span>
            <n-radio-group v-model:value="chartType">
              <n-radio-button value="line">折线图</n-radio-button>
              <n-radio-button value="bar">柱状图</n-radio-button>
            </n-radio-group>
          </div>
        </div>
      </div>

      <div v-if="loading" class="usage-empty usage-loading">
        <n-spin />
        <p>正在加载数据。</p>
      </div>
      <div v-else-if="chartData.length > 0" ref="tokenChartRef" class="chart-container" />
      <div v-else class="usage-empty">
        <p>当前所选周期没有可绘制的趋势数据。</p>
      </div>
    </section>

    <template v-if="!loading && summaryRows.length > 0">
      <section class="surface-panel account-detail-panel">
        <div class="account-detail-head">
          <h2 class="panel-heading">账号明细</h2>
          <span class="account-detail-count">{{ summaryRows.length }} 个账号</span>
        </div>

        <div class="account-detail-grid">
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
                <span>请求次数</span>
                <strong>{{ row.request_count.toLocaleString() }}</strong>
              </div>
            </div>

            <div class="account-detail-share">
              <div class="account-detail-share-track">
                <span
                  class="account-detail-share-segment input"
                  :style="{ width: segmentWidth(row.input_tokens, row.total_tokens) }"
                />
                <span
                  class="account-detail-share-segment output"
                  :style="{ width: segmentWidth(row.output_tokens, row.total_tokens) }"
                />
              </div>
              <span class="account-detail-share-note">
                占整体 Token {{ formatShare(row.total_tokens, totalTokensInPeriod) }}
              </span>
            </div>
          </article>
        </div>
      </section>
    </template>

    <section v-else-if="!loading" class="surface-panel empty-panel">
      <p>当前所选周期内还没有用量数据。</p>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick, onUnmounted } from 'vue'
import * as echarts from 'echarts/core'
import { LineChart, BarChart } from 'echarts/charts'
import {
  GridComponent,
  TooltipComponent,
  LegendComponent,
  DataZoomComponent,
} from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import { useAccountStore } from '@/stores/account'
import { useUsageStore } from '@/stores/usage'
import type { ChartDataPoint, UsagePeriod } from '@/types'
import { resolveAccountDisplayName } from '@/utils/account-display'

echarts.use([
  LineChart,
  BarChart,
  GridComponent,
  TooltipComponent,
  LegendComponent,
  DataZoomComponent,
  CanvasRenderer,
])

interface UsageSummaryRow {
  account_id: string
  account_name: string
  total_tokens: number
  input_tokens: number
  output_tokens: number
  request_count: number
}

const accountStore = useAccountStore()
const usageStore = useUsageStore()

const selectedPeriod = ref<UsagePeriod>('month')
const chartType = ref<'line' | 'bar'>('line')
const loading = ref(false)

const tokenChartRef = ref<HTMLElement | null>(null)
let tokenChart: echarts.ECharts | null = null
let chartResizeObserver: ResizeObserver | null = null

const accountIds = computed(() => accountStore.accounts.map((account) => account.id))
const totalAccountCount = computed(() => accountStore.accounts.length)
const availableAccountCount = computed(
  () =>
    accountStore.accounts.filter(
      (account) => account.is_active && !['error', 'expired'].includes(account.status),
    ).length,
)

const summary = computed(() =>
  usageStore.getSummaryForAccounts(accountIds.value, selectedPeriod.value),
)

const todaySummary = computed(() =>
  usageStore.getSummaryForAccounts(accountIds.value, 'day'),
)

const totalTokensInPeriod = computed(
  () => (summary.value?.total_input_tokens ?? 0) + (summary.value?.total_output_tokens ?? 0),
)

const chartData = computed<ChartDataPoint[]>(() =>
  usageStore.getChartDataForAccounts(accountIds.value, selectedPeriod.value),
)

const todayTotalTokens = computed(
  () => (todaySummary.value?.total_input_tokens ?? 0) + (todaySummary.value?.total_output_tokens ?? 0),
)

const summaryRows = computed<UsageSummaryRow[]>(() =>
  accountStore.accounts
    .map((account) => {
      const accountSummary = usageStore.getSummary(account.id, selectedPeriod.value)
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
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(2)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`
  return String(value)
}

function formatShare(value: number, total: number): string {
  if (total <= 0) {
    return '0%'
  }

  const share = (value / total) * 100
  return `${share.toFixed(share >= 10 ? 0 : 1)}%`
}

function segmentWidth(value: number, total: number): string {
  if (total <= 0) {
    return '0%'
  }

  return `${(value / total) * 100}%`
}

const CHART_COLORS = {
  input: '#0071e3',
  output: '#4b5563',
}

async function loadData() {
  if (accountIds.value.length === 0) return

  loading.value = true
  let shouldRenderChart = false
  try {
    await usageStore.loadUsageForAccounts(accountIds.value, selectedPeriod.value)
    if (selectedPeriod.value !== 'day') {
      await usageStore.loadUsageForAccounts(accountIds.value, 'day')
    }
    shouldRenderChart = true
  } finally {
    loading.value = false
  }

  if (!shouldRenderChart) {
    return
  }

  await nextTick()
  if (chartResizeObserver && tokenChartRef.value) {
    chartResizeObserver.disconnect()
    chartResizeObserver.observe(tokenChartRef.value)
  }
  renderCharts()
}

function onPeriodChange() {
  void loadData()
}

function getBaseChartOptions() {
  return {
    backgroundColor: 'transparent',
    textStyle: {
      color: 'rgba(29, 29, 31, 0.72)',
      fontFamily:
        '"SF Pro Text", "SF Pro Display", -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", "Helvetica Neue", Arial, sans-serif',
    },
    tooltip: {
      trigger: 'axis',
      backgroundColor: '#ffffff',
      borderColor: 'rgba(29, 29, 31, 0.08)',
      borderWidth: 1,
      textStyle: { color: '#1d1d1f' },
      extraCssText: 'box-shadow: rgba(0, 0, 0, 0.16) 0 12px 30px;',
    },
    legend: {
      textStyle: { color: 'rgba(29, 29, 31, 0.72)' },
      top: 0,
    },
    grid: { left: 48, right: 18, top: 48, bottom: 42 },
    xAxis: {
      type: 'category',
      data: chartData.value.map((item) => item.date),
      axisLine: { lineStyle: { color: 'rgba(29, 29, 31, 0.12)' } },
      axisTick: { show: false },
      axisLabel: { color: 'rgba(29, 29, 31, 0.56)', fontSize: 11 },
    },
    yAxis: {
      type: 'value',
      splitLine: { lineStyle: { color: 'rgba(29, 29, 31, 0.08)' } },
      axisLabel: { color: 'rgba(29, 29, 31, 0.56)', fontSize: 11 },
    },
    dataZoom:
      chartData.value.length > 14
        ? [
            { type: 'inside' },
            {
              type: 'slider',
              height: 18,
              bottom: 5,
              borderColor: 'rgba(29, 29, 31, 0.08)',
              backgroundColor: 'rgba(29, 29, 31, 0.04)',
              fillerColor: 'rgba(0, 113, 227, 0.14)',
              handleStyle: { color: '#0071e3' },
            },
          ]
        : [],
  }
}

function renderCharts() {
  if (!tokenChartRef.value || chartData.value.length === 0) return

  if (!tokenChart) tokenChart = echarts.init(tokenChartRef.value)

  const baseOptions = getBaseChartOptions()
  const seriesType = chartType.value

  tokenChart.setOption(
    {
      ...baseOptions,
      series: [
        {
          name: '输入 Token',
          type: seriesType,
          data: chartData.value.map((item) => item.input_tokens),
          smooth: true,
          symbol: 'circle',
          symbolSize: 6,
          lineStyle: { color: CHART_COLORS.input, width: 2 },
          itemStyle: { color: CHART_COLORS.input },
          areaStyle:
            seriesType === 'line'
              ? {
                  color: {
                    type: 'linear',
                    x: 0,
                    y: 0,
                    x2: 0,
                    y2: 1,
                    colorStops: [
                      { offset: 0, color: 'rgba(0, 113, 227, 0.18)' },
                      { offset: 1, color: 'rgba(0, 113, 227, 0)' },
                    ],
                  },
                }
              : undefined,
        },
        {
          name: '输出 Token',
          type: seriesType,
          data: chartData.value.map((item) => item.output_tokens),
          smooth: true,
          symbol: 'circle',
          symbolSize: 6,
          lineStyle: { color: CHART_COLORS.output, width: 2 },
          itemStyle: { color: CHART_COLORS.output },
          areaStyle:
            seriesType === 'line'
              ? {
                  color: {
                    type: 'linear',
                    x: 0,
                    y: 0,
                    x2: 0,
                    y2: 1,
                    colorStops: [
                      { offset: 0, color: 'rgba(75, 85, 99, 0.16)' },
                      { offset: 1, color: 'rgba(75, 85, 99, 0)' },
                    ],
                  },
                }
              : undefined,
        },
      ],
    },
    true,
  )
}

watch(chartType, () => {
  renderCharts()
})

onMounted(async () => {
  if (accountStore.accounts.length === 0) {
    await accountStore.loadAccounts()
  }

  if (accountIds.value.length > 0) {
    await loadData()
  }

  chartResizeObserver = new ResizeObserver(() => {
    tokenChart?.resize()
  })

  if (tokenChartRef.value) chartResizeObserver.observe(tokenChartRef.value)
})

onUnmounted(() => {
  chartResizeObserver?.disconnect()
  tokenChart?.dispose()
})
</script>

<style scoped>
.toolbar-header {
  display: flex;
  align-items: center;
  gap: 12px;
}

.control-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.control-label {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
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

.dashboard-summary-card {
  --card-accent: var(--app-blue);
  --card-background: rgba(0, 113, 227, 0.08);
  --card-border: rgba(0, 113, 227, 0.14);
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
  padding: 16px;
  border-radius: 22px;
  border: 1px solid var(--card-border);
  background: var(--card-background);
  box-shadow: var(--app-shadow);
}

.dashboard-summary-card .metric-label {
  color: var(--card-accent);
}

.metric-total-card {
  --card-accent: #4f46e5;
  --card-background: rgba(79, 70, 229, 0.1);
  --card-border: rgba(79, 70, 229, 0.16);
}

.metric-available-card {
  --card-accent: #248a3d;
  --card-background: rgba(52, 199, 89, 0.12);
  --card-border: rgba(52, 199, 89, 0.18);
}

.metric-today-card {
  --card-accent: #b7791f;
  --card-background: rgba(245, 158, 11, 0.14);
  --card-border: rgba(245, 158, 11, 0.18);
}

.metric-input-card {
  --card-accent: #0071e3;
  --card-background: rgba(0, 113, 227, 0.12);
  --card-border: rgba(0, 113, 227, 0.18);
}

.metric-output-card {
  --card-accent: #4b5563;
  --card-background: rgba(75, 85, 99, 0.1);
  --card-border: rgba(75, 85, 99, 0.14);
}

.metric-request-card {
  --card-accent: #7c3aed;
  --card-background: rgba(124, 58, 237, 0.1);
  --card-border: rgba(124, 58, 237, 0.16);
}

.dashboard-chart-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.dashboard-chart-title {
  font-size: 18px;
}

.dashboard-chart-controls {
  display: flex;
  align-items: flex-start;
  justify-content: flex-end;
  gap: 12px;
  flex-wrap: wrap;
}

.trend-panel {
  gap: 14px;
}

.chart-container {
  width: 100%;
  height: 280px;
}

.usage-empty {
  min-height: 120px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
}

.usage-empty p {
  margin: 0;
  color: var(--app-ink-secondary);
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
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.account-detail-card {
  display: grid;
  gap: 14px;
  padding: 16px;
  border-radius: 22px;
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
  font-size: 18px;
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
.account-detail-metric span,
.account-detail-share-note {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
}

.account-detail-card-total strong {
  font-size: 22px;
  line-height: 1.2;
  color: var(--app-ink);
}

.account-detail-metric-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}

.account-detail-metric {
  display: grid;
  gap: 4px;
  padding: 12px;
  border-radius: 18px;
  background: var(--app-surface);
}

.account-detail-metric strong {
  font-family: var(--font-display);
  font-size: 18px;
  line-height: 1.24;
  color: var(--app-ink);
}

.account-detail-share {
  display: grid;
  gap: 8px;
}

.account-detail-share-track {
  display: flex;
  overflow: hidden;
  height: 8px;
  border-radius: 999px;
  background: rgba(29, 29, 31, 0.08);
}

.account-detail-share-segment {
  display: block;
  height: 100%;
}

.account-detail-share-segment.input {
  background: rgba(0, 113, 227, 0.82);
}

.account-detail-share-segment.output {
  background: rgba(29, 29, 31, 0.68);
}

@media (max-width: 960px) {
  .dashboard-summary-row,
  .dashboard-chart-head,
  .dashboard-chart-controls {
    grid-template-columns: 1fr;
    flex-direction: column;
    align-items: stretch;
  }

  .account-detail-head,
  .account-detail-card-head {
    flex-direction: column;
    align-items: stretch;
  }

  .account-detail-grid,
  .account-detail-metric-grid {
    grid-template-columns: 1fr;
  }

  .account-detail-card-total {
    text-align: left;
  }
}
</style>
