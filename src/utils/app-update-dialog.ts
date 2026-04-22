import { h } from 'vue'
import type { DialogApi, MessageApi } from 'naive-ui'
import {
  installAppUpdate,
  useAvailableAppUpdate,
  type AvailableAppUpdate,
} from '@/utils/app-updater'
import { renderMarkdownLite } from '@/utils/markdown-lite'
import { normalizeUpdateChangelogBody } from '@/utils/update-changelog'

export interface AppUpdateDialogDependencies {
  dialog: DialogApi
  message: MessageApi
  setLoadingState?: (loading: boolean) => void
  openExternalLink?: (url: string) => void | Promise<void>
}

export function showAppUpdateInstallDialog(
  dependencies: AppUpdateDialogDependencies,
  payload?: AvailableAppUpdate | null,
) {
  const { availableAppUpdate } = useAvailableAppUpdate()
  const pendingUpdate = payload ?? availableAppUpdate.value

  if (!pendingUpdate) {
    dependencies.message.success('当前已是最新版本')
    return
  }

  dependencies.dialog.info({
    title: `发现新版本 ${pendingUpdate.version}`,
    content: () =>
      renderUpdateChangelogDialogContent(pendingUpdate.body, dependencies.openExternalLink),
    positiveText: '下载并重启',
    negativeText: '稍后处理',
    onPositiveClick: async () => {
      dependencies.setLoadingState?.(true)
      try {
        const outcome = await installAppUpdate()
        if (outcome.status === 'not_available') {
          dependencies.message.success('当前已是最新版本')
        }
      } catch (error) {
        console.warn('安装更新失败', error)
        dependencies.message.error('安装更新失败，请稍后再试')
      } finally {
        dependencies.setLoadingState?.(false)
      }
    },
  })
}

export function renderUpdateChangelogDialogContent(
  body?: string,
  openExternalLink?: (url: string) => void | Promise<void>,
) {
  const markdownBody = normalizeUpdateChangelogBody(body)

  return h(
    'div',
    {
      class: 'changelog-dialog-content',
      style: {
        maxHeight: '300px',
        overflow: 'auto',
        margin: '0',
        fontFamily: 'var(--font-sans)',
        fontSize: '13px',
        lineHeight: '1.6',
        color: 'var(--app-ink)',
      },
    },
    [
      renderMarkdownLite(markdownBody, (url) => {
        if (openExternalLink) {
          void openExternalLink(url)
          return
        }

        if (typeof window !== 'undefined') {
          window.open(url, '_blank', 'noopener,noreferrer')
        }
      }),
    ],
  )
}
