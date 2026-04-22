import { appendFileSync, readFileSync } from 'node:fs'
import { join } from 'node:path'

const releaseTag = process.argv[2]

if (!releaseTag) {
  throw new Error('缺少发布版本参数，期望传入类似 v0.1.3 的标签名。')
}

const changelogPath = join(process.cwd(), 'CHANGELOG.md')
const normalizedVersion = releaseTag.replace(/^v/i, '')
const changelogMarkdown = readFileSync(changelogPath, 'utf8').replace(/\r\n/g, '\n')
const releaseNotes = extractVersionChangelogSection(changelogMarkdown, normalizedVersion)

if (!releaseNotes) {
  throw new Error(`在 CHANGELOG.md 中未找到版本 ${normalizedVersion} 的更新说明。`)
}

const releaseBody = [
  releaseNotes,
  '',
  '稳定更新入口：',
  'https://github.com/weimin96/CodexX/releases/latest/download/latest.json',
  '',
  '请在下方 Assets 中下载对应平台的安装包。',
].join('\n')

writeMultilineOutput('release_body', releaseBody)
process.stdout.write(releaseBody)

/**
 * 工作流运行在纯 Node 环境，不能直接加载前端 TypeScript 模块。
 * 这里保留与前端一致的版本节提取规则，确保 GitHub Release 与应用内更新说明口径一致。
 */
function extractVersionChangelogSection(markdown, version) {
  const escapedVersion = escapeRegExp(version)
  const headingPattern = new RegExp(
    `^##\\s+(?:\\[)?v?${escapedVersion}(?:\\])?(?:\\s|$).*`,
    'im',
  )
  const headingMatch = headingPattern.exec(markdown)
  if (!headingMatch || headingMatch.index === undefined) {
    return null
  }

  const sectionStart = headingMatch.index + headingMatch[0].length
  const remainingContent = markdown.slice(sectionStart)
  const nextSectionIndex = remainingContent.search(/^##\s+/m)
  const sectionContent =
    nextSectionIndex >= 0 ? remainingContent.slice(0, nextSectionIndex) : remainingContent
  const trimmedSection = sectionContent.trim()

  return trimmedSection.length > 0 ? trimmedSection : null
}

function writeMultilineOutput(name, value) {
  const githubOutputPath = process.env.GITHUB_OUTPUT
  if (!githubOutputPath) {
    return
  }

  const delimiter = `release_notes_${Date.now()}`
  appendFileSync(githubOutputPath, `${name}<<${delimiter}\n${value}\n${delimiter}\n`, 'utf8')
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}
