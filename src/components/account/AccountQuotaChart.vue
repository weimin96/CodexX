<template>
  <div class="quota-grid">
    <div
      v-for="item in quotaItems"
      :key="item.label"
      class="quota-ring-card"
      :class="{ featured }"
    >
      <div class="quota-head">
        <span class="quota-title">{{ item.label }}</span>
        <span class="quota-caption">{{ item.remainingLabel }}</span>
      </div>

      <VChart class="quota-ring" :option="item.option" autoresize />

      <div class="quota-meta">
        <div class="quota-meta-row">
          <span class="quota-meta-label">已用</span>
          <span class="quota-meta-value">{{ item.usedLabel }}</span>
        </div>
        <div class="quota-meta-row">
          <span class="quota-meta-label">重置</span>
          <span class="quota-meta-value">{{ item.resetLabel }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { format } from 'date-fns'
import VChart from 'vue-echarts'
import * as echarts from 'echarts/core'
import { GaugeChart } from 'echarts/charts'
import { GraphicComponent, TooltipComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import type { CodexUsageWindow } from '@/types'

echarts.use([GaugeChart, GraphicComponent, TooltipComponent, CanvasRenderer])

const props = defineProps<{
  fiveHour?: CodexUsageWindow
  oneWeek?: CodexUsageWindow
  featured?: boolean
}>()

interface QuotaRingViewModel {
  label: string
  usedLabel: string
  remainingLabel: string
  resetLabel: string
  option: NonNullable<Parameters<echarts.ECharts['setOption']>[0]>
}

const featured = computed(() => Boolean(props.featured))

const quotaItems = computed<QuotaRingViewModel[]>(() => [
  buildQuotaRing('5 小时', props.fiveHour, '#0071e3', featured.value),
  buildQuotaRing('周', props.oneWeek, '#8b5cf6', featured.value),
])

function buildQuotaRing(
  label: string,
  window: CodexUsageWindow | undefined,
  ringColor: string,
  isFeatured: boolean,
): QuotaRingViewModel {
  const usedPercent = normalizePercent(window?.used_percent)
  const remainingPercent = usedPercent === null ? null : Math.max(0, 100 - usedPercent)
  const textColor = isFeatured ? '#ffffff' : '#1d1d1f'
  const subTextColor = isFeatured ? 'rgba(255, 255, 255, 0.68)' : 'rgba(29, 29, 31, 0.68)'
  const backgroundColor = isFeatured ? 'rgba(255, 255, 255, 0.12)' : 'rgba(29, 29, 31, 0.08)'

  return {
    label,
    usedLabel: usedPercent === null ? '未同步' : `${usedPercent.toFixed(1)}%`,
    remainingLabel: remainingPercent === null ? '未同步' : `剩余 ${remainingPercent.toFixed(1)}%`,
    resetLabel: formatResetTime(window?.reset_at),
    option: {
      animation: false,
      backgroundColor: 'transparent',
      tooltip: {
        trigger: 'item',
        backgroundColor: isFeatured ? '#101114' : '#ffffff',
        borderColor: isFeatured ? 'rgba(255, 255, 255, 0.1)' : 'rgba(29, 29, 31, 0.08)',
        borderWidth: 1,
        textStyle: { color: textColor },
        formatter: remainingPercent === null
          ? `${label}<br/>尚未同步额度`
          : `${label}<br/>剩余 ${remainingPercent.toFixed(1)}%<br/>已用 ${usedPercent?.toFixed(1)}%`,
      },
      graphic: [
        {
          type: 'text',
          left: 'center',
          top: '42%',
          style: {
            text: remainingPercent === null ? '--' : `${remainingPercent.toFixed(0)}%`,
            textAlign: 'center',
            fill: textColor,
            fontSize: 20,
            fontWeight: 700,
            fontFamily:
              '"SF Pro Display", "SF Pro Text", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
          },
        },
        {
          type: 'text',
          left: 'center',
          top: '60%',
          style: {
            text: '剩余',
            textAlign: 'center',
            fill: subTextColor,
            fontSize: 11,
          },
        },
      ],
      series: [
        {
          type: 'gauge',
          radius: '88%',
          center: ['50%', '50%'],
          startAngle: 90,
          endAngle: -270,
          min: 0,
          max: 100,
          splitNumber: 100,
          pointer: { show: false },
          progress: {
            show: true,
            roundCap: true,
            width: 16,
            itemStyle: {
              color: ringColor,
            },
          },
          axisLine: {
            roundCap: true,
            lineStyle: {
              width: 16,
              color: [[1, backgroundColor]],
            },
          },
          axisTick: { show: false },
          splitLine: { show: false },
          axisLabel: { show: false },
          anchor: { show: false },
          detail: { show: false },
          data: [
            {
              value: normalizePercent(window?.used_percent) ?? 0,
            },
          ],
        },
      ],
    },
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
    return '未同步'
  }

  return format(new Date(value * 1000), 'MM-dd HH:mm')
}
</script>

<style scoped>
.quota-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}

.quota-ring-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
  padding: 12px;
  border-radius: 18px;
  background: var(--app-surface-muted);
}

.quota-ring-card.featured {
  background: var(--app-feature-surface-muted);
}

.quota-head {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.quota-title {
  font-size: 12px;
  line-height: 1.33;
  font-weight: 600;
  color: var(--app-ink);
}

.quota-caption {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
}

.quota-ring-card.featured .quota-title {
  color: var(--app-feature-ink);
}

.quota-ring-card.featured .quota-caption {
  color: var(--app-feature-ink-tertiary);
}

.quota-ring {
  width: 100%;
  height: 126px;
}

.quota-meta {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.quota-meta-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.quota-meta-label {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
}

.quota-meta-value {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-secondary);
}

.quota-ring-card.featured .quota-meta-label,
.quota-ring-card.featured .quota-meta-value {
  color: var(--app-feature-ink-tertiary);
}

@media (max-width: 640px) {
  .quota-grid {
    grid-template-columns: 1fr;
  }
}
</style>
