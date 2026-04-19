import { STATUS_LABELS } from '@/types'
import type { Account, AccountStatus, AuthType } from '@/types'

type AccountStatusDisplaySource = Pick<
  Account,
  'auth_type' | 'last_checked_at' | 'status' | 'status_message'
>

export interface AccountStatusDisplay {
  tone: AccountStatus
  label: string
  title: string
}

const NETWORK_ERROR_PATTERNS = ['无法连接', '网络错误', 'error sending request', 'SSL']

function normalizeStatusMessage(value?: string | null): string | null {
  const normalized = value?.trim()
  return normalized ? normalized : null
}

function requiresManualConfirmation(authType: AuthType, message: string | null): boolean {
  return authType === 'cookie_session' || message === 'Session 验证需要手动确认'
}

function isTransportFailure(message: string | null): boolean {
  return Boolean(
    message && NETWORK_ERROR_PATTERNS.some((pattern) => message.includes(pattern)),
  )
}

function isUnexpectedInterfaceResult(message: string | null): boolean {
  return Boolean(message && /^HTTP \d{3}$/.test(message))
}

// 这里显式区分“未检测”和“接口无法判定”，避免把网络异常、人工确认等场景都压成同一个“未知”。
export function resolveAccountStatusDisplay(
  account?: AccountStatusDisplaySource | null,
): AccountStatusDisplay {
  if (!account) {
    return {
      tone: 'unknown',
      label: '未检测',
      title: '尚未执行状态检测',
    }
  }

  const message = normalizeStatusMessage(account.status_message)

  if (!account.last_checked_at) {
    return {
      tone: 'unknown',
      label: '未检测',
      title: '尚未执行状态检测',
    }
  }

  if (account.status !== 'unknown') {
    const label = STATUS_LABELS[account.status]
    return {
      tone: account.status,
      label,
      title: account.status === 'normal' ? label : message ?? label,
    }
  }

  if (requiresManualConfirmation(account.auth_type, message)) {
    return {
      tone: 'unknown',
      label: '需手动确认',
      title: '当前接口无法自动校验 Session 登录状态',
    }
  }

  if (isTransportFailure(message)) {
    return {
      tone: 'unknown',
      label: '接口不可达',
      title: '检测请求未到达认证接口，当前结果不能代表账号失效',
    }
  }

  if (isUnexpectedInterfaceResult(message)) {
    return {
      tone: 'unknown',
      label: '待确认',
      title: '认证接口返回未识别结果，请稍后重试',
    }
  }

  return {
    tone: 'unknown',
    label: STATUS_LABELS.unknown,
    title: '当前接口暂时无法判定账号状态',
  }
}

// 页面文案不直接复用底层接口原文，先给出领域解释，再把原始返回保留为诊断信息。
export function resolveAccountStatusMessage(
  account?: AccountStatusDisplaySource | null,
): string | null {
  if (!account?.last_checked_at) {
    return null
  }

  const message = normalizeStatusMessage(account.status_message)
  if (account.status === 'normal') {
    return null
  }

  if (account.status !== 'unknown') {
    return message
  }

  if (requiresManualConfirmation(account.auth_type, message)) {
    return '当前接口无法自动校验 Session 登录状态，请在 ChatGPT 或 Codex 中确认当前账号仍可用。'
  }

  if (isTransportFailure(message)) {
    return '检测请求未到达认证接口，当前结果不能代表账号失效。请先检查本机代理、证书或网络，再重新检测。'
  }

  if (isUnexpectedInterfaceResult(message)) {
    return '认证接口返回了未识别结果，当前无法据此判定账号状态，请稍后重试。'
  }

  return message ?? '当前接口暂时无法判定账号状态，请稍后重试。'
}

export function resolveAccountStatusDiagnostic(
  account?: AccountStatusDisplaySource | null,
): string | null {
  const message = normalizeStatusMessage(account?.status_message)
  if (!message || account?.status !== 'unknown' || !account.last_checked_at) {
    return null
  }

  if (requiresManualConfirmation(account.auth_type, message)) {
    return null
  }

  return `接口返回：${message}`
}
