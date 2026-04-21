import { defineStore } from 'pinia'
import { ref } from 'vue'
import { usageService } from '@/services'
import type { UsageSummary, ChartDataPoint, UsagePeriod, UsageQuery } from '@/types'

export const useUsageStore = defineStore('usage', () => {
  const summaries = ref<Map<string, UsageSummary>>(new Map())
  const chartData = ref<Map<string, ChartDataPoint[]>>(new Map())
  const period = ref<UsagePeriod>('month')
  const loading = ref(false)

  function cacheKey(accountId: string, p: UsagePeriod) {
    return `${accountId}:${p}`
  }

  function normalizeAccountIds(accountIds: string[]): string[] {
    return [...new Set(accountIds.filter(Boolean))]
  }

  async function readUsageData(accountId: string, p: UsagePeriod) {
    const query = buildUsageQuery(accountId, p)
    const [summary, chart] = await Promise.all([
      usageService.getUsageStats(query),
      usageService.getUsageChartData(query),
    ])
    summaries.value.set(cacheKey(accountId, p), summary)
    chartData.value.set(cacheKey(accountId, p), chart)
  }

  async function refreshUsageData(accountId: string, p: UsagePeriod) {
    await usageService.fetchUsage(accountId)
    await readUsageData(accountId, p)
  }

  async function loadUsage(accountId: string, p: UsagePeriod = period.value) {
    loading.value = true
    try {
      await refreshUsageData(accountId, p)
      period.value = p
    } finally {
      loading.value = false
    }
  }

  async function loadUsageForAccounts(accountIds: string[], p: UsagePeriod = period.value) {
    loading.value = true
    try {
      await refreshUsageForAccounts(accountIds, p)
    } finally {
      loading.value = false
    }
  }

  async function loadCachedUsageForAccounts(
    accountIds: string[],
    p: UsagePeriod = period.value,
  ) {
    for (const accountId of normalizeAccountIds(accountIds)) {
      await readUsageData(accountId, p)
    }
    period.value = p
  }

  async function refreshUsageForAccounts(
    accountIds: string[],
    p: UsagePeriod = period.value,
  ) {
    for (const accountId of normalizeAccountIds(accountIds)) {
      await refreshUsageData(accountId, p)
    }
    period.value = p
  }

  function hasCachedUsage(accountId: string, p: UsagePeriod = period.value): boolean {
    const key = cacheKey(accountId, p)
    return summaries.value.has(key) || chartData.value.has(key)
  }

  function hasCachedUsageForAccounts(
    accountIds: string[],
    p: UsagePeriod = period.value,
  ): boolean {
    return normalizeAccountIds(accountIds).some((accountId) => hasCachedUsage(accountId, p))
  }

  function getSummary(accountId: string, p: UsagePeriod = period.value): UsageSummary | null {
    return summaries.value.get(cacheKey(accountId, p)) ?? null
  }

  function getChartData(accountId: string, p: UsagePeriod = period.value): ChartDataPoint[] {
    return chartData.value.get(cacheKey(accountId, p)) ?? []
  }

  function getSummaryForAccounts(
    accountIds: string[],
    p: UsagePeriod = period.value,
  ): UsageSummary | null {
    const uniqueAccountIds = normalizeAccountIds(accountIds)
    if (uniqueAccountIds.length === 0) {
      return null
    }

    let hasLoadedSummary = false
    const combined = uniqueAccountIds.reduce<UsageSummary>(
      (current, accountId) => {
        const summary = getSummary(accountId, p)
        if (!summary) {
          return current
        }

        hasLoadedSummary = true
        current.total_input_tokens += summary.total_input_tokens
        current.total_output_tokens += summary.total_output_tokens
        current.total_requests += summary.total_requests
        current.total_cost += summary.total_cost
        return current
      },
      {
        account_id: 'all',
        period: p,
        total_input_tokens: 0,
        total_output_tokens: 0,
        total_requests: 0,
        total_cost: 0,
      },
    )

    return hasLoadedSummary ? combined : null
  }

  function getChartDataForAccounts(
    accountIds: string[],
    p: UsagePeriod = period.value,
  ): ChartDataPoint[] {
    const pointMap = new Map<string, ChartDataPoint>()

    for (const accountId of normalizeAccountIds(accountIds)) {
      for (const point of getChartData(accountId, p)) {
        const existingPoint = pointMap.get(point.date)
        if (existingPoint) {
          existingPoint.input_tokens += point.input_tokens
          existingPoint.output_tokens += point.output_tokens
          existingPoint.request_count += point.request_count
          existingPoint.cost += point.cost
          continue
        }

        pointMap.set(point.date, { ...point })
      }
    }

    return [...pointMap.values()].sort((left, right) => left.date.localeCompare(right.date))
  }

  function setPeriod(p: UsagePeriod) {
    period.value = p
  }

  function buildUsageQuery(accountId: string, p: UsagePeriod): UsageQuery {
    return {
      account_id: accountId,
      period: p,
      timezone_offset_minutes: resolveLocalTimezoneOffsetMinutes(),
    }
  }

  function resolveLocalTimezoneOffsetMinutes(): number {
    return -new Date().getTimezoneOffset()
  }

  return {
    period,
    loading,
    loadUsage,
    loadUsageForAccounts,
    loadCachedUsageForAccounts,
    refreshUsageForAccounts,
    getSummary,
    getChartData,
    getSummaryForAccounts,
    getChartDataForAccounts,
    hasCachedUsageForAccounts,
    setPeriod,
  }
})
