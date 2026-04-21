<template>
  <section class="surface-panel section-grid heatmap-panel">
    <div class="heatmap-head">
      <div class="heatmap-copy">
        <h2 class="panel-heading heatmap-title">Token用量</h2>
      </div>

      <div class="heatmap-meta">
        <div class="heatmap-stat-card">
          <span>活跃天数</span>
          <strong>{{ activeDayCount.toLocaleString() }}</strong>
        </div>
        <div class="heatmap-stat-card heatmap-peak-card">
          <span>峰值日</span>
          <strong>{{ peakDayLabel }}</strong>
        </div>
        <div class="heatmap-legend">
          <span class="heatmap-legend-text">少</span>
          <span
            v-for="level in heatmapLegendLevels"
            :key="`legend-${level}`"
            class="heatmap-legend-cell"
            :class="`level-${level}`"
          />
          <span class="heatmap-legend-text">多</span>
        </div>
      </div>
    </div>

    <div v-if="loading" class="usage-empty usage-loading">
      <n-spin />
      <p>正在加载数据。</p>
    </div>

    <template v-else>
      <p v-if="activeDayCount === 0" class="heatmap-empty-note">
        最近一年还没有 Token 记录，当前格子均为零用量。
      </p>

      <div class="heatmap-board">
        <div class="heatmap-corner" aria-hidden="true" />

        <div class="heatmap-month-row">
          <span
            v-for="slot in heatmapMonthSlots"
            :key="`month-${slot.key}`"
            class="heatmap-month-slot"
          >
            {{ slot.label }}
          </span>
        </div>

        <div class="heatmap-weekday-labels" aria-hidden="true">
          <span
            v-for="(label, index) in heatmapWeekdayLabels"
            :key="`weekday-${index}`"
            class="heatmap-weekday-label"
          >
            {{ label }}
          </span>
        </div>

        <div class="heatmap-week-columns">
          <div
            v-for="week in heatmapWeeks"
            :key="week.key"
            class="heatmap-week-column"
          >
            <span
              v-for="cell in week.cells"
              :key="cell.key"
              class="heatmap-cell"
              :class="[
                `level-${cell.level}`,
                {
                  'outside-range': !cell.in_range,
                  today: cell.is_today,
                },
              ]"
              :title="buildCellTitle(cell)"
              :aria-label="buildCellTitle(cell)"
            />
          </div>
        </div>
      </div>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { ChartDataPoint } from '@/types'

interface HeatmapCell {
  key: string
  date: string
  total_tokens: number
  input_tokens: number
  output_tokens: number
  request_count: number
  in_range: boolean
  is_today: boolean
  level: number
}

interface HeatmapWeek {
  key: string
  cells: HeatmapCell[]
}

interface HeatmapMonthSlot {
  key: string
  label: string
}

const props = defineProps<{
  loading: boolean
  dailyData: ChartDataPoint[]
}>()

const heatmapLegendLevels = [0, 1, 2, 3, 4] as const
const heatmapWeekdayLabels = ['一', '', '三', '', '五', '', '']

const today = startOfDay(new Date())
const todayKey = formatDateKey(today)

const annualRange = computed(() => {
  const end = today
  const start = addDays(end, -364)
  return {
    start,
    end,
    grid_start: startOfWeek(start),
    grid_end: endOfWeek(end),
  }
})

const dailyPointMap = computed(() => {
  const pointMap = new Map<string, ChartDataPoint>()
  for (const point of props.dailyData) {
    pointMap.set(point.date, point)
  }
  return pointMap
})

const positiveTotals = computed(() =>
  props.dailyData
    .map((point) => point.input_tokens + point.output_tokens)
    .filter((value) => value > 0)
    .sort((left, right) => left - right),
)

const heatmapThresholds = computed(() => resolveHeatmapThresholds(positiveTotals.value))

const heatmapWeeks = computed<HeatmapWeek[]>(() => {
  const { start, end, grid_start, grid_end } = annualRange.value
  const weeks: HeatmapWeek[] = []
  let cursor = new Date(grid_start)

  while (cursor <= grid_end) {
    const weekStart = new Date(cursor)
    const cells: HeatmapCell[] = []

    for (let dayOffset = 0; dayOffset < 7; dayOffset += 1) {
      const currentDate = addDays(weekStart, dayOffset)
      const currentKey = formatDateKey(currentDate)
      const inRange = currentDate >= start && currentDate <= end
      const point = inRange ? dailyPointMap.value.get(currentKey) : undefined
      const totalTokens = (point?.input_tokens ?? 0) + (point?.output_tokens ?? 0)

      cells.push({
        key: `${weekStart.toISOString()}-${dayOffset}`,
        date: currentKey,
        total_tokens: totalTokens,
        input_tokens: point?.input_tokens ?? 0,
        output_tokens: point?.output_tokens ?? 0,
        request_count: point?.request_count ?? 0,
        in_range: inRange,
        is_today: currentKey === todayKey,
        level: inRange ? resolveHeatmapLevel(totalTokens, heatmapThresholds.value) : 0,
      })
    }

    weeks.push({
      key: formatDateKey(weekStart),
      cells,
    })
    cursor = addDays(weekStart, 7)
  }

  return weeks
})

const heatmapMonthSlots = computed<HeatmapMonthSlot[]>(() => {
  const slots = heatmapWeeks.value.map((week) => ({
    key: week.key,
    label: '',
  }))
  const monthRanges = new Map<
    string,
    {
      key: string
      label: string
      start_week_index: number
      end_week_index: number
    }
  >()

  heatmapWeeks.value.forEach((week, weekIndex) => {
    for (const cell of week.cells) {
      if (!cell.in_range) {
        continue
      }

      const date = parseDateKey(cell.date)
      const key = monthKey(date)
      const existingRange = monthRanges.get(key)
      if (existingRange) {
        existingRange.end_week_index = weekIndex
        continue
      }

      monthRanges.set(key, {
        key,
        label: formatMonthLabel(date),
        start_week_index: weekIndex,
        end_week_index: weekIndex,
      })
    }
  })

  const visibleMonthRanges = Array.from(monthRanges.values())
  const lastMonthRange = visibleMonthRanges[visibleMonthRanges.length - 1]

  visibleMonthRanges.forEach((range, index) => {
    const weekSpan = range.end_week_index - range.start_week_index + 1
    const sameAsEndMonth = Boolean(
      lastMonthRange && range.key !== lastMonthRange.key && range.label === lastMonthRange.label,
    )
    if (index === 0 && (weekSpan < 2 || sameAsEndMonth)) {
      return
    }

    slots[range.start_week_index].label = range.label
  })

  return slots
})

const activeDayCount = computed(() =>
  props.dailyData.filter((point) => point.input_tokens + point.output_tokens > 0).length,
)

const peakDay = computed<HeatmapCell | null>(() => {
  let candidate: HeatmapCell | null = null

  for (const week of heatmapWeeks.value) {
    for (const cell of week.cells) {
      if (!cell.in_range || cell.total_tokens <= 0) {
        continue
      }

      if (!candidate || cell.total_tokens > candidate.total_tokens) {
        candidate = cell
      }
    }
  }

  return candidate
})

const peakDayLabel = computed(() => {
  if (!peakDay.value) {
    return '暂无'
  }

  return `${formatMonthDay(peakDay.value.date)} · ${formatTokens(peakDay.value.total_tokens)}`
})

function buildCellTitle(cell: HeatmapCell): string {
  if (!cell.in_range) {
    return ''
  }

  return [
    cell.date,
    `总 Token：${formatTokens(cell.total_tokens)}`,
    `输入：${formatTokens(cell.input_tokens)}`,
    `输出：${formatTokens(cell.output_tokens)}`,
    `请求：${cell.request_count.toLocaleString()}`,
  ].join('\n')
}

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

function resolveHeatmapThresholds(positiveValues: number[]): [number, number, number] {
  if (positiveValues.length === 0) {
    return [0, 0, 0]
  }

  const sortedValues = [...positiveValues].sort((left, right) => left - right)
  const q1 = pickQuantile(sortedValues, 0.25)
  const q2 = pickQuantile(sortedValues, 0.5)
  const q3 = pickQuantile(sortedValues, 0.75)

  if (q1 === q3) {
    const maxValue = sortedValues[sortedValues.length - 1]
    return [maxValue * 0.25, maxValue * 0.5, maxValue * 0.75]
  }

  return [q1, q2, q3]
}

function pickQuantile(sortedValues: number[], quantile: number): number {
  const index = Math.min(
    sortedValues.length - 1,
    Math.floor((sortedValues.length - 1) * quantile),
  )
  return sortedValues[index]
}

function resolveHeatmapLevel(totalTokens: number, thresholds: [number, number, number]): number {
  if (totalTokens <= 0) {
    return 0
  }

  if (totalTokens <= thresholds[0]) {
    return 1
  }

  if (totalTokens <= thresholds[1]) {
    return 2
  }

  if (totalTokens <= thresholds[2]) {
    return 3
  }

  return 4
}

function startOfDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate())
}

function startOfWeek(date: Date): Date {
  const normalizedDate = startOfDay(date)
  const dayOffset = (normalizedDate.getDay() + 6) % 7
  return addDays(normalizedDate, -dayOffset)
}

function endOfWeek(date: Date): Date {
  const normalizedDate = startOfDay(date)
  const dayOffset = 6 - ((normalizedDate.getDay() + 6) % 7)
  return addDays(normalizedDate, dayOffset)
}

function addDays(date: Date, dayOffset: number): Date {
  const nextDate = new Date(date)
  nextDate.setDate(nextDate.getDate() + dayOffset)
  return startOfDay(nextDate)
}

function formatDateKey(date: Date): string {
  const year = date.getFullYear()
  const month = `${date.getMonth() + 1}`.padStart(2, '0')
  const day = `${date.getDate()}`.padStart(2, '0')
  return `${year}-${month}-${day}`
}

function parseDateKey(value: string): Date {
  const [year, month, day] = value.split('-').map((segment) => Number(segment))
  return new Date(year, month - 1, day)
}

function monthKey(date: Date): string {
  return `${date.getFullYear()}-${date.getMonth() + 1}`
}

function formatMonthLabel(date: Date): string {
  return `${date.getMonth() + 1}月`
}

function formatMonthDay(dateKey: string): string {
  const date = parseDateKey(dateKey)
  return `${date.getMonth() + 1}月${date.getDate()}日`
}
</script>

<style scoped>
.heatmap-panel {
  gap: 18px;
}

.heatmap-head {
  display: grid;
  gap: 12px;
  align-items: stretch;
}

.heatmap-copy {
  display: grid;
  gap: 4px;
}

.heatmap-title {
  font-size: 18px;
}

.heatmap-empty-note {
  margin: 0;
  font-size: 12px;
  line-height: 1.5;
  color: var(--app-ink-secondary);
}

.heatmap-meta {
  display: flex;
  align-items: flex-start;
  justify-content: flex-start;
  gap: 10px;
  flex-wrap: wrap;
}

.heatmap-stat-card {
  display: grid;
  gap: 4px;
  min-width: 112px;
  padding: 10px 12px;
  border-radius: 16px;
  background: var(--app-surface-muted);
}

.heatmap-stat-card span,
.heatmap-legend-text {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
}

.heatmap-stat-card strong {
  font-family: var(--font-display);
  font-size: 16px;
  line-height: 1.2;
  color: var(--app-ink);
}

.heatmap-peak-card {
  min-width: 160px;
}

.heatmap-legend {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-height: 48px;
  padding: 0 12px;
  border-radius: 16px;
  background: var(--app-surface-muted);
}

.heatmap-legend-cell {
  width: 12px;
  height: 12px;
  border-radius: 3px;
  border: 1px solid transparent;
  box-sizing: border-box;
}

.heatmap-board {
  --heatmap-gap: 4px;
  --heatmap-column-gap: 10px;
  --heatmap-weekday-width: 22px;
  display: grid;
  grid-template-columns: var(--heatmap-weekday-width) minmax(0, 1fr);
  grid-template-rows: 16px auto;
  gap: 8px var(--heatmap-column-gap);
  width: 100%;
  min-width: 0;
  margin-top: 2px;
}

.heatmap-corner {
  width: var(--heatmap-weekday-width);
  height: 16px;
}

.heatmap-month-row {
  display: flex;
  gap: var(--heatmap-gap);
  width: 100%;
  min-width: 0;
}

.heatmap-month-slot {
  flex: 1 1 0;
  min-width: 0;
  font-size: 11px;
  line-height: 1.2;
  color: var(--app-ink-tertiary);
  white-space: nowrap;
}

.heatmap-weekday-labels {
  display: grid;
  grid-template-rows: repeat(7, 1fr);
  row-gap: var(--heatmap-gap);
}

.heatmap-weekday-label {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  font-size: 10px;
  line-height: 1;
  color: var(--app-ink-tertiary);
}

.heatmap-week-columns {
  display: flex;
  gap: var(--heatmap-gap);
  width: 100%;
  min-width: 0;
}

.heatmap-week-column {
  display: grid;
  grid-template-rows: repeat(7, 1fr);
  row-gap: var(--heatmap-gap);
  flex: 1 1 0;
  min-width: 0;
}

.heatmap-cell {
  display: block;
  width: 100%;
  aspect-ratio: 1 / 1;
  min-width: 4px;
  min-height: 4px;
  border-radius: 3px;
  border: 1px solid transparent;
  box-sizing: border-box;
  background: rgba(29, 29, 31, 0.06);
}

.heatmap-cell.level-0,
.heatmap-legend-cell.level-0 {
  background: rgba(29, 29, 31, 0.06);
}

.heatmap-cell.level-1,
.heatmap-legend-cell.level-1 {
  background: #d9f2e4;
}

.heatmap-cell.level-2,
.heatmap-legend-cell.level-2 {
  background: #9ad9b0;
}

.heatmap-cell.level-3,
.heatmap-legend-cell.level-3 {
  background: #58b87d;
}

.heatmap-cell.level-4,
.heatmap-legend-cell.level-4 {
  background: #1d8a49;
}

.heatmap-cell.outside-range {
  background: transparent;
  border-color: transparent;
}

.heatmap-cell.today {
  border-color: rgba(29, 29, 31, 0.28);
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

@media (max-width: 960px) {
  .heatmap-board {
    --heatmap-gap: 3px;
    --heatmap-column-gap: 8px;
    --heatmap-weekday-width: 16px;
  }

  .heatmap-month-slot {
    font-size: 10px;
  }

  .heatmap-weekday-label {
    font-size: 9px;
  }
}
</style>
