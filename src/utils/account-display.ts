import type { Account } from '@/types'

type AccountDisplaySource = Pick<Account, 'name' | 'email' | 'avatar_text'>
type AccountOrganizationSource = Pick<Account, 'organization' | 'updated_at'>

function isLegacyLocalSyncName(name: string): boolean {
  return name.trim().startsWith('本地 auth.json')
}

function extractLegacyWrappedText(value: string): string | null {
  const matched = value.match(/[（(]([^（）()]+)[）)]/)
  const candidate = matched?.[1]?.trim()
  return candidate || null
}

function resolveLegacyPreferredName(value?: string | null): string | null {
  const candidate = value?.trim()
  if (!candidate) {
    return null
  }

  return candidate
}

function isLocalSyncOrganization(organization?: string | null): boolean {
  return organization?.trim().startsWith('本地文件同步') ?? false
}

function formatDisplayTimestamp(iso: string): string | null {
  const timestamp = new Date(iso)
  if (Number.isNaN(timestamp.getTime())) {
    return null
  }

  const year = timestamp.getFullYear()
  const month = `${timestamp.getMonth() + 1}`.padStart(2, '0')
  const day = `${timestamp.getDate()}`.padStart(2, '0')
  const hours = `${timestamp.getHours()}`.padStart(2, '0')
  const minutes = `${timestamp.getMinutes()}`.padStart(2, '0')

  return `${year}-${month}-${day} ${hours}:${minutes}`
}

export function resolveAccountDisplayName(account: AccountDisplaySource): string {
  const normalizedEmail = account.email?.trim()
  if (normalizedEmail) {
    return normalizedEmail
  }

  const normalizedName = account.name.trim()
  if (!normalizedName) {
    return '未命名账号'
  }

  if (!isLegacyLocalSyncName(normalizedName)) {
    return normalizedName
  }

  return (
    resolveLegacyPreferredName(extractLegacyWrappedText(normalizedName)) ||
    resolveLegacyPreferredName(account.email) ||
    '本地同步账号'
  )
}

export function resolveAccountAvatarText(account: AccountDisplaySource): string {
  const normalizedAvatarText = account.avatar_text?.trim()
  const displayName = resolveAccountDisplayName(account)
  return displayName[0]?.toUpperCase() || normalizedAvatarText || '?'
}

export function resolveAccountOrganizationLabel(account: AccountOrganizationSource): string | null {
  const organization = account.organization?.trim()
  if (!organization) {
    return null
  }

  return isLocalSyncOrganization(organization) ? '同步信息' : '组织'
}

export function resolveAccountOrganizationDisplay(
  account: AccountOrganizationSource,
): string | null {
  const organization = account.organization?.trim()
  if (!organization) {
    return null
  }

  if (!isLocalSyncOrganization(organization)) {
    return organization
  }

  const formattedTimestamp = formatDisplayTimestamp(account.updated_at)
  return formattedTimestamp ? `本地同步 · ${formattedTimestamp}` : '本地同步'
}
