<template>
  <div class="view-container">
    <!-- Header -->
    <div class="view-header">
      <div>
        <h1 class="view-title">用量统计</h1>
        <p class="view-sub">查看 Token 使用量、请求次数和费用估算</p>
      </div>
      <div class="header-actions">
        <!-- Period selector -->
        <n-radio-group v-model:value="selectedPeriod" size="small" @update:value="onPeriodChange">
          <n-radio-button value="day">今日</n-radio-button>
          <n-radio-button value="week">本周</n-radio-button>
          <n-radio-button value="month">本月</n-radio-button>
        </n-radio-group>
        <!-- Account selector -->
        <n-select
          v-model:value="selectedAccountId"
          :options="accountOptions"
          size="small"
          style="width: 180px;"
          @update:value="loadData"
        />
        <n-button size="small" quaternary :loading="loading" @click="loadData">
          刷新
        </n-button>
      </div>
    </div>

    <!-- Summary cards -->
    <div v-if="summary" class="summary-grid">
      <div class="summary-card">
        <div class="summary-icon input">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" stroke="currentColor" stroke-width="1.5"/></svg>
        </div>
        <div class="summary-val">{{ formatTokens(summary.total_input_tokens) }}</div>
        <div class="summary-label">输入 Token</div>
        <div class="summary-sub">{{ periodLabel }}累计</div>
      </div>
      <div class="summary-card">
        <div class="summary-icon output">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2zM8 9h8M8 13h6" stroke="currentColor" stroke-width="1.5"/></svg>
        </div>
        <div class="summary-val">{{ formatTokens(summary.total_output_tokens) }}</div>
        <div class="summary-label">输出 Token</div>
        <div class="summary-sub">{{ periodLabel }}累计</div>
      </div>
      <div class="summary-card">
        <div class="summary-icon requests">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        </div>
        <div class="summary-val">{{ summary.total_requests.toLocaleString() }}</div>
        <div class="summary-label">请求次数</div>
        <div class="summary-sub">{{ periodLabel }}累计</div>
      </div>
      <div class="summary-card highlight">
        <div class="summary-icon cost">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none"><line x1="12" y1="1" x2="12" y2="23" stroke="currentColor" stroke-width="1.5"/><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" stroke="currentColor" stroke-width="1.5"/></svg>
        </div>
        <div class="summary-val">${{ summary.total_cost.toFixed(4) }}</div>
        <div class="summary-label">费用估算</div>
        <div class="summary-sub">{{ periodLabel }}累计</div>
      </div>
    </div>

    <div v-if="loading" class="loading-state">
      <n-spin />
      <span>加载数据中...</span>
    </div>

    <template v-else-if="chartData.length > 0">
      <!-- Token chart -->
      <n-card title="Token 用量趋势" size="small">
        <div class="chart-toolbar">
          <n-radio-group v-model:value="chartType" size="small">
            <n-radio-button value="line">折线图</n-radio-button>
            <n-radio-button value="bar">柱状图</n-radio-button>
          </n-radio-group>
        </div>
        <div ref="tokenChartRef" class="chart-container" />
      </n-card>

      <!-- Cost chart -->
      <n-card title="费用趋势" size="small">
        <div ref="costChartRef" class="chart-container chart-sm" />
      </n-card>

      <!-- Data table -->
      <n-card title="明细数据" size="small">
        <n-data-table
          :columns="tableColumns"
          :data="chartData"
          :pagination="{ pageSize: 10 }"
          size="small"
          striped
        />
      </n-card>
    </template>

    <div v-else class="empty-state">
      <p>暂无用量数据</p>
      <n-button size="small" @click="loadData">重新加载</n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick, onUnmounted } from 'vue'
import * as echarts from 'echarts/core'
import { LineChart, BarChart } from 'echarts/charts'
import {
  GridComponent, TooltipComponent, LegendComponent,
  DataZoomComponent,
} from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import type { DataTableColumns } from 'naive-ui'
import { useAccountStore } from '@/stores/account'
import { useUsageStore } from '@/stores/usage'
import type { ChartDataPoint, UsagePeriod } from '@/types'

echarts.use([LineChart, BarChart, GridComponent, TooltipComponent, LegendComponent, DataZoomComponent, CanvasRenderer])

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

const accountOptions = computed(() =>
  accountStore.accounts.map((a) => ({ label: a.name, value: a.id }))
)

const summary = computed(() =>
  selectedAccountId.value ? usageStore.getSummary(selectedAccountId.value, selectedPeriod.value) : null
)

const chartData = computed<ChartDataPoint[]>(() =>
  selectedAccountId.value ? usageStore.getChartData(selectedAccountId.value, selectedPeriod.value) : []
)

const periodLabel = computed(() => ({ day: '今日', week: '本周', month: '本月' }[selectedPeriod.value]))

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
}

const CHART_COLORS = { input: '#4f8ef7', output: '#18a058', cost: '#f0a020', requests: '#8b5cf6' }

const tableColumns: DataTableColumns<ChartDataPoint> = [
  { title: '日期', key: 'date', sorter: 'default' },
  { title: '输入 Token', key: 'input_tokens', render: (row) => formatTokens(row.input_tokens), sorter: (a, b) => a.input_tokens - b.input_tokens },
  { title: '输出 Token', key: 'output_tokens', render: (row) => formatTokens(row.output_tokens), sorter: (a, b) => a.output_tokens - b.output_tokens },
  { title: '请求次数', key: 'request_count', sorter: (a, b) => a.request_count - b.request_count },
  { title: '费用 ($)', key: 'cost', render: (row) => row.cost.toFixed(6), sorter: (a, b) => a.cost - b.cost },
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
  loadData()
}

function getBaseChartOpts() {
  return {
    backgroundColor: 'transparent',
    textStyle: { color: '#8b949e', fontFamily: 'Lato' },
    tooltip: {
      trigger: 'axis',
      backgroundColor: '#1c2333',
      borderColor: '#30363d',
      textStyle: { color: '#e6edf3' },
    },
    legend: { textStyle: { color: '#8b949e' }, top: 0 },
    grid: { left: 48, right: 16, top: 40, bottom: 40 },
    xAxis: {
      type: 'category',
      data: chartData.value.map((d) => d.date),
      axisLine: { lineStyle: { color: '#30363d' } },
      axisTick: { show: false },
      axisLabel: { color: '#8b949e', fontSize: 11 },
    },
    yAxis: {
      type: 'value',
      splitLine: { lineStyle: { color: '#21262d' } },
      axisLabel: { color: '#8b949e', fontSize: 11 },
    },
    dataZoom: chartData.value.length > 14
      ? [{ type: 'inside' }, { type: 'slider', height: 20, bottom: 5, borderColor: '#30363d', backgroundColor: '#161b22', fillerColor: 'rgba(79,142,247,0.12)', handleStyle: { color: '#4f8ef7' } }]
      : [],
  }
}

function renderCharts() {
  if (!tokenChartRef.value || !costChartRef.value) return

  // Token chart
  if (!tokenChart) tokenChart = echarts.init(tokenChartRef.value, 'dark')
  const seriesType = chartType.value

  tokenChart.setOption({
    ...getBaseChartOpts(),
    series: [
      {
        name: '输入 Token',
        type: seriesType,
        data: chartData.value.map((d) => d.input_tokens),
        smooth: true,
        symbol: 'circle',
        symbolSize: 4,
        lineStyle: { color: CHART_COLORS.input, width: 2 },
        itemStyle: { color: CHART_COLORS.input },
        areaStyle: seriesType === 'line' ? { color: { type: 'linear', x: 0, y: 0, x2: 0, y2: 1, colorStops: [{ offset: 0, color: 'rgba(79,142,247,0.2)' }, { offset: 1, color: 'rgba(79,142,247,0)' }] } } : undefined,
      },
      {
        name: '输出 Token',
        type: seriesType,
        data: chartData.value.map((d) => d.output_tokens),
        smooth: true,
        symbol: 'circle',
        symbolSize: 4,
        lineStyle: { color: CHART_COLORS.output, width: 2 },
        itemStyle: { color: CHART_COLORS.output },
        areaStyle: seriesType === 'line' ? { color: { type: 'linear', x: 0, y: 0, x2: 0, y2: 1, colorStops: [{ offset: 0, color: 'rgba(24,160,88,0.2)' }, { offset: 1, color: 'rgba(24,160,88,0)' }] } } : undefined,
      },
    ],
  }, true)

  // Cost chart
  if (!costChart) costChart = echarts.init(costChartRef.value, 'dark')
  costChart.setOption({
    ...getBaseChartOpts(),
    legend: undefined,
    yAxis: { ...getBaseChartOpts().yAxis, axisLabel: { ...getBaseChartOpts().yAxis.axisLabel, formatter: (v: number) => `$${v.toFixed(4)}` } },
    series: [
      {
        name: '费用',
        type: 'bar',
        data: chartData.value.map((d) => d.cost),
        itemStyle: {
          color: { type: 'linear', x: 0, y: 0, x2: 0, y2: 1, colorStops: [{ offset: 0, color: '#f0a020' }, { offset: 1, color: 'rgba(240,160,32,0.3)' }] },
          borderRadius: [4, 4, 0, 0],
        },
      },
    ],
  }, true)
}

// Re-render when chart type changes
watch(chartType, () => {
  renderCharts()
})

onMounted(async () => {
  if (accountStore.accounts.length === 0) await accountStore.loadAccounts()
  const active = accountStore.activeAccount
  if (active) {
    selectedAccountId.value = active.id
    await loadData()
  } else if (accountOptions.value.length > 0) {
    selectedAccountId.value = accountOptions.value[0].value
    await loadData()
  }

  // Resize observer
  const observer = new ResizeObserver(() => {
    tokenChart?.resize()
    costChart?.resize()
  })
  if (tokenChartRef.value) observer.observe(tokenChartRef.value)
  if (costChartRef.value) observer.observe(costChartRef.value)
})

onUnmounted(() => {
  tokenChart?.dispose()
  costChart?.dispose()
})
</script>

<style scoped>
.view-container { padding: 24px; display: flex; flex-direction: column; gap: 16px; max-width: 1100px; }

.view-header {
  display: flex; align-items: flex-start; justify-content: space-between; gap: 16px;
}
.view-title { font-size: 22px; font-weight: 700; color: var(--text-primary); letter-spacing: -0.3px; }
.view-sub { font-size: 13px; color: var(--text-secondary); margin-top: 2px; }
.header-actions { display: flex; align-items: center; gap: 8px; flex-shrink: 0; flex-wrap: wrap; }

.summary-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
}

.summary-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  position: relative;
  overflow: hidden;
}
.summary-card.highlight {
  border-color: rgba(240,160,32,0.3);
  background: linear-gradient(135deg, rgba(240,160,32,0.06) 0%, var(--bg-secondary) 60%);
}

.summary-icon {
  width: 36px; height: 36px; border-radius: 8px;
  display: flex; align-items: center; justify-content: center;
  margin-bottom: 8px;
}
.summary-icon.input   { background: rgba(79,142,247,0.15); color: #4f8ef7; }
.summary-icon.output  { background: rgba(24,160,88,0.15); color: #18a058; }
.summary-icon.requests { background: rgba(139,92,246,0.15); color: #8b5cf6; }
.summary-icon.cost    { background: rgba(240,160,32,0.15); color: #f0a020; }

.summary-val {
  font-size: 26px; font-weight: 700; color: var(--text-primary);
  font-family: 'Fira Code', monospace; line-height: 1;
}

.summary-label { font-size: 13px; color: var(--text-secondary); font-weight: 500; }
.summary-sub { font-size: 11px; color: var(--text-secondary); opacity: 0.6; }

.chart-toolbar { display: flex; justify-content: flex-end; margin-bottom: 8px; }

.chart-container { width: 100%; height: 300px; }
.chart-sm { height: 200px; }

.loading-state {
  display: flex; flex-direction: column; align-items: center;
  justify-content: center; gap: 12px; padding: 80px 0;
  color: var(--text-secondary); font-size: 14px;
}

.empty-state {
  display: flex; flex-direction: column; align-items: center;
  justify-content: center; gap: 12px; padding: 80px 0;
  color: var(--text-secondary);
}
</style>
