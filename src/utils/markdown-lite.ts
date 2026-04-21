import type { VNodeChild } from 'vue'
import { h } from 'vue'

export interface MarkdownLiteLinkHandler {
  (url: string): void
}

/**
 * 轻量 Markdown 渲染器（受控子集）
 *
 * 设计取舍：
 * - 仅覆盖项目内更新日志常见格式（标题、段落、无序列表、代码块、行内代码、链接）。
 * - 不生成不受控 HTML，不使用 v-html，避免把外部更新日志内容作为可执行的 DOM 注入。
 * - 不支持表格/图片/HTML 内嵌等复杂能力；遇到未知语法会按普通文本降级展示。
 */
export function renderMarkdownLite(markdown: string, onOpenLink?: MarkdownLiteLinkHandler): VNodeChild {
  const normalizedMarkdown = normalizeMarkdown(markdown)
  const lines = normalizedMarkdown.split('\n')

  const blocks: VNodeChild[] = []
  let index = 0

  while (index < lines.length) {
    const line = lines[index]
    if (isBlankLine(line)) {
      index += 1
      continue
    }

    const fenceStart = tryParseFenceStart(line)
    if (fenceStart) {
      const outcome = consumeFenceBlock(lines, index)
      blocks.push(renderFenceBlock(outcome.code, fenceStart.language))
      index = outcome.nextIndex
      continue
    }

    const heading = tryParseHeading(line)
    if (heading) {
      blocks.push(renderHeading(heading.level, heading.text))
      index += 1
      continue
    }

    if (isUnorderedListItem(line)) {
      const outcome = consumeUnorderedList(lines, index, onOpenLink)
      blocks.push(outcome.vnode)
      index = outcome.nextIndex
      continue
    }

    const paragraphOutcome = consumeParagraph(lines, index, onOpenLink)
    blocks.push(paragraphOutcome.vnode)
    index = paragraphOutcome.nextIndex
  }

  return h(
    'div',
    {
      class: 'markdown-lite-root',
    },
    blocks,
  )
}

function normalizeMarkdown(markdown: string): string {
  return (markdown ?? '').replace(/\r\n/g, '\n')
}

function isBlankLine(value: string): boolean {
  return value.trim().length === 0
}

function tryParseFenceStart(line: string): { language: string | null } | null {
  const match = /^```(\S+)?\s*$/.exec(line.trim())
  if (!match) {
    return null
  }

  return { language: match[1] ? match[1] : null }
}

function consumeFenceBlock(lines: string[], startIndex: number): { code: string; nextIndex: number } {
  const codeLines: string[] = []
  let index = startIndex + 1

  while (index < lines.length) {
    const line = lines[index]
    if (/^```\s*$/.test(line.trim())) {
      return { code: codeLines.join('\n'), nextIndex: index + 1 }
    }
    codeLines.push(line)
    index += 1
  }

  return { code: codeLines.join('\n'), nextIndex: lines.length }
}

function renderFenceBlock(code: string, language: string | null): VNodeChild {
  const languageLabel = language ? `语言：${language}` : null
  return h('div', { class: 'markdown-lite-fence' }, [
    languageLabel ? h('div', { class: 'markdown-lite-fence-language' }, languageLabel) : null,
    h('pre', { class: 'markdown-lite-pre' }, h('code', { class: 'markdown-lite-code' }, code)),
  ])
}

function tryParseHeading(line: string): { level: 1 | 2 | 3; text: string } | null {
  const trimmed = line.trim()
  const match = /^(#{1,3})\s+(.+)$/.exec(trimmed)
  if (!match) {
    return null
  }

  const level = match[1].length
  if (level !== 1 && level !== 2 && level !== 3) {
    return null
  }

  return { level: level as 1 | 2 | 3, text: match[2].trim() }
}

function renderHeading(level: 1 | 2 | 3, text: string): VNodeChild {
  const tag = level === 1 ? 'h2' : level === 2 ? 'h3' : 'h4'
  const className = level === 1 ? 'markdown-lite-h1' : level === 2 ? 'markdown-lite-h2' : 'markdown-lite-h3'
  return h(tag, { class: className }, text)
}

function isUnorderedListItem(line: string): boolean {
  return /^\s*-\s+/.test(line)
}

function consumeUnorderedList(
  lines: string[],
  startIndex: number,
  onOpenLink?: MarkdownLiteLinkHandler,
): { vnode: VNodeChild; nextIndex: number } {
  const items: VNodeChild[] = []
  let index = startIndex

  while (index < lines.length) {
    const line = lines[index]
    if (!isUnorderedListItem(line)) {
      break
    }

    const content = line.replace(/^\s*-\s+/, '')
    items.push(h('li', { class: 'markdown-lite-li' }, renderInlineMarkdown(content, onOpenLink)))
    index += 1
  }

  return { vnode: h('ul', { class: 'markdown-lite-ul' }, items), nextIndex: index }
}

function consumeParagraph(
  lines: string[],
  startIndex: number,
  onOpenLink?: MarkdownLiteLinkHandler,
): { vnode: VNodeChild; nextIndex: number } {
  const paragraphLines: string[] = []
  let index = startIndex

  while (index < lines.length) {
    const line = lines[index]
    if (isBlankLine(line)) {
      break
    }
    if (tryParseFenceStart(line) || tryParseHeading(line) || isUnorderedListItem(line)) {
      break
    }

    paragraphLines.push(line.trim())
    index += 1
  }

  const paragraphText = paragraphLines.join(' ')
  return {
    vnode: h('p', { class: 'markdown-lite-p' }, renderInlineMarkdown(paragraphText, onOpenLink)),
    nextIndex: index,
  }
}

function renderInlineMarkdown(text: string, onOpenLink?: MarkdownLiteLinkHandler): VNodeChild[] {
  const nodes: VNodeChild[] = []
  let cursor = 0

  while (cursor < text.length) {
    const codeMatch = findNextInlineCode(text, cursor)
    const linkMatch = findNextInlineLink(text, cursor)
    const nextMatch = pickNextMatch(codeMatch, linkMatch)

    if (!nextMatch) {
      nodes.push(text.slice(cursor))
      break
    }

    if (nextMatch.start > cursor) {
      nodes.push(text.slice(cursor, nextMatch.start))
    }

    if (nextMatch.type === 'code') {
      nodes.push(h('code', { class: 'markdown-lite-inline-code' }, nextMatch.content))
    } else {
      nodes.push(
        h(
          'a',
          {
            class: 'markdown-lite-link',
            href: nextMatch.url,
            rel: 'noopener noreferrer',
            target: '_blank',
            onClick: (event: MouseEvent) => {
              if (!onOpenLink) {
                return
              }
              event.preventDefault()
              onOpenLink(nextMatch.url)
            },
          },
          nextMatch.label,
        ),
      )
    }

    cursor = nextMatch.end
  }

  return nodes
}

type InlineMatch =
  | { type: 'code'; start: number; end: number; content: string }
  | { type: 'link'; start: number; end: number; label: string; url: string }

function findNextInlineCode(text: string, startIndex: number): InlineMatch | null {
  const start = text.indexOf('`', startIndex)
  if (start < 0) {
    return null
  }

  const end = text.indexOf('`', start + 1)
  if (end < 0) {
    return null
  }

  const content = text.slice(start + 1, end)
  return { type: 'code', start, end: end + 1, content }
}

function findNextInlineLink(text: string, startIndex: number): InlineMatch | null {
  const startBracket = text.indexOf('[', startIndex)
  if (startBracket < 0) {
    return null
  }

  const endBracket = text.indexOf(']', startBracket + 1)
  if (endBracket < 0 || endBracket + 1 >= text.length || text[endBracket + 1] !== '(') {
    return null
  }

  const endParen = text.indexOf(')', endBracket + 2)
  if (endParen < 0) {
    return null
  }

  const label = text.slice(startBracket + 1, endBracket)
  const url = text.slice(endBracket + 2, endParen).trim()
  if (!isSafeExternalUrl(url)) {
    return null
  }

  return { type: 'link', start: startBracket, end: endParen + 1, label, url }
}

function pickNextMatch(codeMatch: InlineMatch | null, linkMatch: InlineMatch | null): InlineMatch | null {
  if (codeMatch && linkMatch) {
    return codeMatch.start <= linkMatch.start ? codeMatch : linkMatch
  }
  return codeMatch ?? linkMatch
}

function isSafeExternalUrl(url: string): boolean {
  return /^https?:\/\//i.test(url)
}

