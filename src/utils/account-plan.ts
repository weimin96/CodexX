export type AccountPlanTone = 'green' | 'blue' | 'purple' | 'neutral'

function normalizePlanType(planType?: string | null): string {
  return planType?.trim().toLowerCase() || ''
}

export function formatAccountPlanType(planType?: string | null): string {
  const normalized = normalizePlanType(planType)
  return normalized ? normalized.toUpperCase() : '未知计划'
}

export function resolveAccountPlanTone(planType?: string | null): AccountPlanTone {
  const normalized = normalizePlanType(planType)
  if (normalized === 'free') {
    return 'green'
  }
  if (normalized === 'plus') {
    return 'blue'
  }
  if (normalized === 'pro') {
    return 'purple'
  }
  return 'neutral'
}

export function supportsFiveHourQuota(planType?: string | null): boolean {
  return normalizePlanType(planType) !== 'free'
}
