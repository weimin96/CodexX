import { defineStore } from 'pinia'
import { ref } from 'vue'
import { usageService } from '@/services'
import type { UsageSummary, ChartDataPoint, UsagePeriod } from '@/types'

export const useUsageStore = defineStore('usage', () => {
  const summaries = ref<Map<string, UsageSummary>>(new Map())
  const chartData = ref<Map<string, ChartDataPoint[]>>(new Map())
  const period = ref<UsagePeriod>('month')
  const loading = ref(false)

  function cacheKey(accountId: string, p: UsagePeriod) {
    return `${accountId}:${p}`
  }

  async function loadUsage(accountId: string, p: UsagePeriod = period.value) {
    loading.value = true
    try {
      const query = { account_id: accountId, period: p }
      const [summary, chart] = await Promise.all([
        usageService.getUsageStats(query),
        usageService.getUsageChartData(query),
      ])
      summaries.value.set(cacheKey(accountId, p), summary)
      chartData.value.set(cacheKey(accountId, p), chart)
    } finally {
      loading.value = false
    }
  }

  function getSummary(accountId: string, p: UsagePeriod = period.value): UsageSummary | null {
    return summaries.value.get(cacheKey(accountId, p)) ?? null
  }

  function getChartData(accountId: string, p: UsagePeriod = period.value): ChartDataPoint[] {
    return chartData.value.get(cacheKey(accountId, p)) ?? []
  }

  function setPeriod(p: UsagePeriod) {
    period.value = p
  }

  return {
    period,
    loading,
    loadUsage,
    getSummary,
    getChartData,
    setPeriod,
  }
})
