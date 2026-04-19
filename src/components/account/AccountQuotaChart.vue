<template>
  <div class="quota-line-card" :class="{ featured }">
    <div class="quota-summary">
      <div v-for="item in quotaItems" :key="item.label" class="quota-summary-item">
        <span class="quota-title">{{ item.title }}</span>
        <strong class="quota-value">{{ item.valueLabel }}</strong>
      </div>
    </div>

    <VChart class="quota-line-chart" :option="chartOption" autoresize />

    <div class="quota-reset-line">
      <span v-for="item in quotaItems" :key="`${item.label}-reset`" class="quota-reset-item">
        <span class="quota-reset-label">{{ item.label }}重置</span>
        <span class="quota-reset-value">{{ item.resetLabel }}</span>
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { format } from 'date-fns'
import VChart from 'vue-echarts'
import * as echarts from 'echarts/core'
import { LineChart } from 'echarts/charts'
import { GridComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import type { CodexUsageWindow } from '@/types'
import { supportsFiveHourQuota } from '@/utils/account-plan'

echarts.use([LineChart, GridComponent, TooltipComponent, CanvasRenderer])

const props = defineProps<{
  fiveHour?: CodexUsageWindow
  oneWeek?: CodexUsageWindow
  planType?: string
  featured?: boolean
}>()

interface QuotaLineViewModel {
  label: string
  title: string
  xStart: number
  xEnd: number
  color: string
  trackColor: string
  usedLabel: string
  valueLabel: string
  resetLabel: string
  progressEnd: number
  remainingPercent: number | null
}

const featured = computed(() => Boolean(props.featured))

const quotaItems = computed<QuotaLineViewModel[]>(() => {
  const definitions: Array<{
    label: string
    title: string
    window: CodexUsageWindow | undefined
    color: string
    trackColor: string
  }> = []

  if (supportsFiveHourQuota(props.planType)) {
    definitions.push({
      label: '5小时',
      title: '5小时剩余',
      window: props.fiveHour,
      color: '#34c759',
      trackColor: 'rgba(52, 199, 89, 0.18)',
    })
  }

  definitions.push({
    label: '7天',
    title: '7天剩余',
    window: props.oneWeek,
    color: '#0071e3',
    trackColor: 'rgba(0, 113, 227, 0.16)',
  })

  const positions =
    definitions.length === 1
      ? [{ xStart: 0, xEnd: 200 }]
      : [
          { xStart: 0, xEnd: 96 },
          { xStart: 104, xEnd: 200 },
        ]

  return definitions.map((definition, index) =>
    buildQuotaLine(
      definition.label,
      definition.title,
      definition.window,
      definition.color,
      definition.trackColor,
      positions[index].xStart,
      positions[index].xEnd,
    ),
  )
})

const chartOption = computed<NonNullable<Parameters<echarts.ECharts['setOption']>[0]>>(() => {
  const items = quotaItems.value
  const textColor = featured.value ? '#ffffff' : '#1d1d1f'

  return {
    animation: false,
    backgroundColor: 'transparent',
    grid: {
      left: 0,
      right: 0,
      top: 4,
      bottom: 4,
      containLabel: false,
    },
    tooltip: {
      trigger: 'item',
      backgroundColor: featured.value ? '#101114' : '#ffffff',
      borderColor: featured.value ? 'rgba(255, 255, 255, 0.1)' : 'rgba(29, 29, 31, 0.08)',
      borderWidth: 1,
      textStyle: { color: textColor },
      formatter: (params: unknown) => {
        const seriesName =
          typeof params === 'object' && params && 'seriesName' in params
            ? String(params.seriesName)
            : ''
        const item = items.find((quotaItem) => quotaItem.label === seriesName)
        if (!item) return ''

        if (item.remainingPercent === null) {
          return `${item.label}<br/>尚未提供额度`
        }

        return `${item.label}<br/>剩余 ${item.remainingPercent.toFixed(1)}%<br/>${item.usedLabel}`
      },
    },
    xAxis: {
      type: 'value',
      min: 0,
      max: 200,
      show: false,
    },
    yAxis: {
      type: 'value',
      min: -0.5,
      max: 0.5,
      show: false,
    },
    series: items.flatMap((item) => [
      {
        name: `${item.label}轨道`,
        type: 'line',
        data: [
          [item.xStart, 0],
          [item.xEnd, 0],
        ],
        symbol: 'none',
        silent: true,
        lineStyle: {
          width: 5,
          color: item.trackColor,
        },
        z: 1,
      },
      {
        name: item.label,
        type: 'line',
        data: [
          [item.xStart, 0],
          [item.progressEnd, 0],
        ],
        symbol: 'none',
        lineStyle: {
          width: 5,
          color: item.color,
        },
        z: 2,
      },
    ]),
  }
})

function buildQuotaLine(
  label: string,
  title: string,
  window: CodexUsageWindow | undefined,
  color: string,
  trackColor: string,
  xStart: number,
  xEnd: number,
): QuotaLineViewModel {
  const usedPercent = normalizePercent(window?.used_percent)
  const remainingPercent = usedPercent === null ? null : Math.max(0, 100 - usedPercent)
  const progressEnd = xStart + ((xEnd - xStart) * (remainingPercent ?? 0)) / 100

  return {
    label,
    title,
    xStart,
    xEnd,
    color,
    trackColor,
    usedLabel: usedPercent === null ? '已用 未提供' : `已用 ${usedPercent.toFixed(1)}%`,
    valueLabel: remainingPercent === null ? '未提供' : `${remainingPercent.toFixed(0)}%`,
    resetLabel: formatResetTime(window?.reset_at),
    progressEnd,
    remainingPercent,
  }
}

function normalizePercent(value?: number): number | null {
  if (!Number.isFinite(value)) {
    return null
  }

  return Math.max(0, Math.min(100, Number(value)))
}

function formatResetTime(value?: number): string {
  if (!Number.isFinite(value) || !value || value <= 0) {
    return '未提供'
  }

  return format(new Date(value * 1000), 'yyyy/MM/dd HH:mm')
}
</script>

<style scoped>
.quota-line-card {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
  padding: 10px 12px;
  border-radius: 14px;
  background: var(--app-surface-muted);
}

.quota-line-card.featured {
  background: var(--app-feature-surface-muted);
}

.quota-summary {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(0, 1fr));
  gap: 10px;
}

.quota-summary-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  min-width: 0;
}

.quota-title {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
  white-space: nowrap;
}

.quota-value {
  min-width: 0;
  font-size: 11px;
  line-height: 1.33;
  font-weight: 600;
  color: var(--app-ink);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  text-align: right;
}

.quota-line-card.featured .quota-title {
  color: var(--app-feature-ink-tertiary);
}

.quota-line-card.featured .quota-value {
  color: var(--app-feature-ink);
}

.quota-line-chart {
  width: 100%;
  height: 18px;
}

.quota-reset-line {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(0, 1fr));
  column-gap: 10px;
  row-gap: 4px;
  font-size: 10px;
  line-height: 1.35;
  color: var(--app-ink-tertiary);
}

.quota-reset-item {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 4px;
  min-width: 0;
}

.quota-reset-label,
.quota-reset-value {
  white-space: nowrap;
}

.quota-reset-label {
  text-align: left;
}

.quota-reset-value {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  text-align: right;
}

.quota-line-card.featured .quota-reset-line {
  color: var(--app-feature-ink-tertiary);
}

@media (max-width: 640px) {
  .quota-summary {
    grid-template-columns: 1fr;
    gap: 4px;
  }

  .quota-reset-line {
    grid-template-columns: 1fr;
  }
}
</style>
