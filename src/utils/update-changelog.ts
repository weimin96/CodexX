const INSTALLED_UPDATE_CHANGELOG_KEY = 'codexx.installed-update-changelog'
const EMPTY_CHANGELOG_TEXT = '该版本没有提供更新日志。'

export interface UpdateChangelogPayload {
  version: string
  body?: string
  installed_at?: string
}

export function normalizeUpdateChangelogBody(body?: string): string {
  const trimmedBody = body?.trim()
  return trimmedBody ? trimmedBody : EMPTY_CHANGELOG_TEXT
}

export function extractVersionChangelog(markdown: string, version: string): string {
  const escapedVersion = escapeRegExp(version)
  const headingPattern = new RegExp(
    `^##\\s+(?:\\[)?v?${escapedVersion}(?:\\])?(?:\\s|$).*`,
    'im',
  )
  const headingMatch = headingPattern.exec(markdown)
  if (!headingMatch || headingMatch.index === undefined) {
    return normalizeUpdateChangelogBody()
  }

  const sectionStart = headingMatch.index + headingMatch[0].length
  const remainingContent = markdown.slice(sectionStart)
  const nextSectionIndex = remainingContent.search(/^##\s+/m)
  const sectionContent =
    nextSectionIndex >= 0 ? remainingContent.slice(0, nextSectionIndex) : remainingContent

  return normalizeUpdateChangelogBody(sectionContent)
}

export function rememberInstalledUpdateChangelog(payload: UpdateChangelogPayload) {
  if (typeof window === 'undefined') {
    return
  }

  try {
    window.localStorage.setItem(
      INSTALLED_UPDATE_CHANGELOG_KEY,
      JSON.stringify({
        version: payload.version,
        body: payload.body,
        installed_at: payload.installed_at ?? new Date().toISOString(),
      }),
    )
  } catch (error) {
    console.warn('记录更新日志失败', error)
  }
}

export function consumeInstalledUpdateChangelog(): UpdateChangelogPayload | null {
  if (typeof window === 'undefined') {
    return null
  }

  const rawRecord = window.localStorage.getItem(INSTALLED_UPDATE_CHANGELOG_KEY)
  if (!rawRecord) {
    return null
  }

  window.localStorage.removeItem(INSTALLED_UPDATE_CHANGELOG_KEY)

  try {
    const parsedRecord = JSON.parse(rawRecord) as Partial<UpdateChangelogPayload>
    if (!parsedRecord.version) {
      return null
    }

    return {
      version: parsedRecord.version,
      body: parsedRecord.body,
      installed_at: parsedRecord.installed_at,
    }
  } catch (error) {
    console.warn('读取更新日志失败', error)
    return null
  }
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}
