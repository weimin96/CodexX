<template>
  <div class="app-page">
    <section class="surface-panel section-grid">
      <div class="toolbar-header">
        <h1 class="panel-heading">用量统计</h1>
      </div>

      <div class="controls-grid">
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

      <div class="metric-grid dashboard-metric-grid">
        <div class="metric-card metric-card-compact">
          <span class="metric-label">总账号数</span>
          <strong class="metric-value">{{ totalAccountCount }}</strong>
        </div>
        <div class="metric-card metric-card-compact">
          <span class="metric-label">今日 Token</span>
          <strong class="metric-value">{{ formatTokens(todayTotalTokens) }}</strong>
        </div>
        <div class="metric-card metric-card-compact">
          <span class="metric-label">输入 Token</span>
          <strong class="metric-value">{{ formatTokens(summary?.total_input_tokens ?? 0) }}</strong>
        </div>
        <div class="metric-card metric-card-compact">
          <span class="metric-label">输出 Token</span>
          <strong class="metric-value">{{ formatTokens(summary?.total_output_tokens ?? 0) }}</strong>
        </div>
        <div class="metric-card metric-card-compact">
          <span class="metric-label">请求次数</span>
          <strong class="metric-value">{{ (summary?.total_requests ?? 0).toLocaleString() }}</strong>
        </div>
      </div>
    </section>

    <section v-if="loading" class="surface-panel empty-panel">
      <n-spin />
      <p>正在加载数据。</p>
    </section>

    <template v-else-if="chartData.length > 0 || summaryRows.length > 0">
      <section class="surface-panel">
        <h2 class="panel-heading">Token 用量趋势</h2>
        <div v-if="chartData.length > 0" ref="tokenChartRef" class="chart-container" />
        <div v-else class="usage-empty">
          <p>当前所选周期没有可绘制的趋势数据。</p>
        </div>
      </section>

      <section class="surface-panel">
        <h2 class="panel-heading">账号明细</h2>
        <n-data-table
          :columns="tableColumns"
          :data="summaryRows"
          :pagination="{ pageSize: 10 }"
          size="small"
          striped
        />
      </section>
    </template>

    <section v-else class="surface-panel empty-panel">
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
import type { DataTableColumns } from 'naive-ui'
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

const summary = computed(() =>
  usageStore.getSummaryForAccounts(accountIds.value, selectedPeriod.value),
)

const todaySummary = computed(() =>
  usageStore.getSummaryForAccounts(accountIds.value, 'day'),
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
        accountSummary.total_output_tokens +
        accountSummary.total_requests
      if (totalTokens <= 0) {
        return null
      }

      return {
        account_id: account.id,
        account_name: resolveAccountDisplayName(account),
        input_tokens: accountSummary.total_input_tokens,
        output_tokens: accountSummary.total_output_tokens,
        request_count: accountSummary.total_requests,
      }
    })
    .filter((row): row is UsageSummaryRow => Boolean(row))
    .sort((left, right) => {
      const leftWeight = left.input_tokens + left.output_tokens + left.request_count
      const rightWeight = right.input_tokens + right.output_tokens + right.request_count
      return rightWeight - leftWeight
    }),
)

function formatTokens(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(2)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`
  return String(value)
}

const CHART_COLORS = {
  input: '#0071e3',
  output: '#1d1d1f',
}

const tableColumns: DataTableColumns<UsageSummaryRow> = [
  {
    title: '账号名',
    key: 'account_name',
    sorter: (first, second) => first.account_name.localeCompare(second.account_name),
  },
  {
    title: '输入 Token',
    key: 'input_tokens',
    render: (row) => formatTokens(row.input_tokens),
    sorter: (first, second) => first.input_tokens - second.input_tokens,
  },
  {
    title: '输出 Token',
    key: 'output_tokens',
    render: (row) => formatTokens(row.output_tokens),
    sorter: (first, second) => first.output_tokens - second.output_tokens,
  },
  {
    title: '请求次数',
    key: 'request_count',
    sorter: (first, second) => first.request_count - second.request_count,
  },
]

async function loadData() {
  if (accountIds.value.length === 0) return

  loading.value = true
  try {
    await usageStore.loadUsageForAccounts(accountIds.value, selectedPeriod.value)
    if (selectedPeriod.value !== 'day') {
      await usageStore.loadUsageForAccounts(accountIds.value, 'day')
    }
    await nextTick()
    if (chartResizeObserver && tokenChartRef.value) {
      chartResizeObserver.observe(tokenChartRef.value)
    }
    renderCharts()
  } finally {
    loading.value = false
  }
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
                      { offset: 0, color: 'rgba(29, 29, 31, 0.12)' },
                      { offset: 1, color: 'rgba(29, 29, 31, 0)' },
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
  gap: 14px;
}

.controls-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
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

.dashboard-metric-grid {
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 12px;
}

.metric-card-compact {
  padding: 14px 16px;
}

.chart-container {
  width: 100%;
  height: 250px;
  margin-top: 14px;
}

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

@media (max-width: 960px) {
  .controls-grid {
    grid-template-columns: 1fr;
  }
}
</style>
