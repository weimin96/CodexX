<template>
  <div class="app-page">
    <section class="page-hero">
      <div class="page-hero-copy">
        <h1 class="page-title">用量统计</h1>
        <p class="page-subtitle">查看趋势与费用。</p>
      </div>
      <div class="hero-stats">
        <div class="hero-stat">
          <span class="hero-stat-label">账号</span>
          <strong class="hero-stat-value">{{ activeAccountLabel }}</strong>
        </div>
        <div class="hero-stat">
          <span class="hero-stat-label">周期</span>
          <strong class="hero-stat-value">{{ periodLabel }}</strong>
        </div>
        <div class="hero-stat">
          <span class="hero-stat-label">记录</span>
          <strong class="hero-stat-value">{{ chartData.length }}</strong>
        </div>
      </div>
    </section>

    <section class="surface-panel section-grid">
      <div class="toolbar-header">
        <div>
          <h2 class="panel-heading">条件</h2>
          <p class="panel-copy">选择账号与时间范围。</p>
        </div>
        <n-button secondary :loading="loading" @click="loadData">刷新数据</n-button>
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
          <span class="control-label">账号</span>
          <n-select
            v-model:value="selectedAccountId"
            :options="accountOptions"
            placeholder="请选择账号"
            @update:value="loadData"
          />
        </div>

        <div class="control-block">
          <span class="control-label">图表类型</span>
          <n-radio-group v-model:value="chartType">
            <n-radio-button value="line">折线图</n-radio-button>
            <n-radio-button value="bar">柱状图</n-radio-button>
          </n-radio-group>
        </div>
      </div>
    </section>

    <section v-if="loading" class="surface-panel empty-panel">
      <n-spin />
      <p>正在加载数据。</p>
    </section>

    <template v-else-if="chartData.length > 0">
      <section class="surface-panel">
        <h2 class="panel-heading">摘要</h2>
        <p class="panel-copy">当前周期汇总。</p>
        <div v-if="summary" class="metric-grid summary-grid">
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
            <strong class="metric-value">{{ summary.total_requests.toLocaleString() }}</strong>
          </div>
          <div class="metric-card">
            <span class="metric-label">费用估算</span>
            <strong class="metric-value">${{ summary.total_cost.toFixed(4) }}</strong>
          </div>
        </div>
      </section>

      <section class="two-column-grid">
        <div class="surface-panel">
          <h2 class="panel-heading">Token 用量趋势</h2>
          <p class="panel-copy">蓝色为输入，深色为输出。</p>
          <div ref="tokenChartRef" class="chart-container" />
        </div>

        <div class="surface-panel surface-panel-dark">
          <h2 class="panel-heading">费用趋势</h2>
          <p class="panel-copy">逐日费用估算。</p>
          <div ref="costChartRef" class="chart-container chart-container-sm" />
        </div>
      </section>

      <section class="surface-panel">
        <h2 class="panel-heading">明细数据</h2>
        <p class="panel-copy">逐日记录。</p>
        <n-data-table
          :columns="tableColumns"
          :data="chartData"
          :pagination="{ pageSize: 10 }"
          size="small"
          striped
        />
      </section>
    </template>

    <section v-else class="surface-panel empty-panel">
      <p>当前账号在所选周期内还没有用量数据。</p>
      <n-button secondary @click="loadData">重新加载</n-button>
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

const accountStore = useAccountStore()
const usageStore = useUsageStore()

const selectedAccountId = ref<string>('')
const selectedPeriod = ref<UsagePeriod>('month')
const chartType = ref<'line' | 'bar'>('line')
const loading = ref(false)

const tokenChartRef = ref<HTMLElement | null>(null)
const costChartRef = ref<HTMLElement | null>(null)
let tokenChart: echarts.ECharts | null = null
let costChart: echarts.ECharts | null = null
let chartResizeObserver: ResizeObserver | null = null

const accountOptions = computed(() =>
  accountStore.accounts.map((account) => ({
    label: resolveAccountDisplayName(account),
    value: account.id,
  })),
)

const summary = computed(() =>
  selectedAccountId.value
    ? usageStore.getSummary(selectedAccountId.value, selectedPeriod.value)
    : null,
)

const chartData = computed<ChartDataPoint[]>(() =>
  selectedAccountId.value
    ? usageStore.getChartData(selectedAccountId.value, selectedPeriod.value)
    : [],
)

const periodLabel = computed(
  () =>
    ({
      day: '今日',
      week: '本周',
      month: '本月',
    })[selectedPeriod.value],
)

const activeAccountLabel = computed(() => {
  if (!selectedAccountId.value) return '未选择'
  const account = accountStore.accounts.find((item) => item.id === selectedAccountId.value)
  return account ? resolveAccountDisplayName(account) : '未知账号'
})

function formatTokens(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(2)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`
  return String(value)
}

const CHART_COLORS = {
  input: '#0071e3',
  output: '#1d1d1f',
  cost: 'rgba(255, 255, 255, 0.82)',
}

const tableColumns: DataTableColumns<ChartDataPoint> = [
  { title: '日期', key: 'date', sorter: 'default' },
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
  {
    title: '费用 ($)',
    key: 'cost',
    render: (row) => row.cost.toFixed(6),
    sorter: (first, second) => first.cost - second.cost,
  },
]

async function loadData() {
  if (!selectedAccountId.value) return
  loading.value = true
  try {
    await usageStore.loadUsage(selectedAccountId.value, selectedPeriod.value)
    await nextTick()
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
  if (!tokenChartRef.value || !costChartRef.value) return

  if (!tokenChart) tokenChart = echarts.init(tokenChartRef.value)
  if (!costChart) costChart = echarts.init(costChartRef.value)

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

  costChart.setOption(
    {
      ...baseOptions,
      tooltip: {
        trigger: 'axis',
        backgroundColor: '#1d1d1f',
        borderColor: 'rgba(255, 255, 255, 0.08)',
        borderWidth: 1,
        textStyle: { color: '#ffffff' },
      },
      legend: undefined,
      xAxis: {
        ...baseOptions.xAxis,
        axisLine: { lineStyle: { color: 'rgba(255, 255, 255, 0.12)' } },
        axisLabel: { color: 'rgba(255, 255, 255, 0.56)', fontSize: 11 },
      },
      yAxis: {
        type: 'value',
        splitLine: { lineStyle: { color: 'rgba(255, 255, 255, 0.08)' } },
        axisLabel: {
          color: 'rgba(255, 255, 255, 0.56)',
          fontSize: 11,
          formatter: (value: number) => `$${value.toFixed(4)}`,
        },
      },
      dataZoom:
        chartData.value.length > 14
          ? [
              { type: 'inside' },
              {
                type: 'slider',
                height: 18,
                bottom: 5,
                borderColor: 'rgba(255, 255, 255, 0.08)',
                backgroundColor: 'rgba(255, 255, 255, 0.04)',
                fillerColor: 'rgba(255, 255, 255, 0.14)',
                handleStyle: { color: '#ffffff' },
              },
            ]
          : [],
      series: [
        {
          name: '费用',
          type: 'bar',
          data: chartData.value.map((item) => item.cost),
          itemStyle: {
            color: CHART_COLORS.cost,
            borderRadius: [8, 8, 0, 0],
          },
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

  const activeAccount = accountStore.activeAccount
  if (activeAccount) {
    selectedAccountId.value = activeAccount.id
    await loadData()
  } else if (accountOptions.value.length > 0) {
    selectedAccountId.value = accountOptions.value[0].value
    await loadData()
  }

  chartResizeObserver = new ResizeObserver(() => {
    tokenChart?.resize()
    costChart?.resize()
  })

  if (tokenChartRef.value) chartResizeObserver.observe(tokenChartRef.value)
  if (costChartRef.value) chartResizeObserver.observe(costChartRef.value)
})

onUnmounted(() => {
  chartResizeObserver?.disconnect()
  tokenChart?.dispose()
  costChart?.dispose()
})
</script>

<style scoped>
.toolbar-header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 14px;
}

.controls-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
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

.summary-grid {
  margin-top: 14px;
}

.chart-container {
  width: 100%;
  height: 250px;
  margin-top: 14px;
}

.chart-container-sm {
  height: 250px;
}

@media (max-width: 960px) {
  .controls-grid {
    grid-template-columns: 1fr;
  }
}
</style>
