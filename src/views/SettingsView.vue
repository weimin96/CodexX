<template>
  <div class="app-page settings-page">
    <section class="surface-panel">
      <div class="settings-section-head">
        <div>
          <h1 class="panel-heading">界面与启动</h1>
        </div>
        <span class="autosave-pill" :class="`autosave-pill-${autosaveState}`">
          {{ autosaveStatusLabel }}
        </span>
      </div>

      <div class="setting-list">
        <div class="setting-item">
          <div class="setting-copy">
            <div class="setting-title">视觉主题</div>
            <div class="setting-description">切换浅色或深色界面。</div>
          </div>
          <n-radio-group
            :value="settingsStore.settings.theme"
            size="small"
            @update:value="handleThemeChange"
          >
            <n-radio-button value="light">浅色</n-radio-button>
            <n-radio-button value="dark">深色</n-radio-button>
          </n-radio-group>
        </div>

        <div class="setting-item">
          <div class="setting-copy">
            <div class="setting-title">开机自启</div>
            <div class="setting-description">系统启动时运行。</div>
          </div>
          <n-switch
            :value="settingsStore.settings.autostart === 'true'"
            @update:value="handleAutostartChange"
          />
        </div>
      </div>
    </section>

    <section class="surface-panel">
      <div class="settings-section-head">
        <div>
          <h2 class="panel-heading">后台任务</h2>
        </div>
      </div>

      <div class="setting-list">

        <div class="setting-item">
          <div class="setting-copy">
            <div class="setting-title">状态检测间隔</div>
            <div class="setting-description">应用运行期间的后台检查频率，会同步刷新账号状态与 Codex 额度。</div>
          </div>
          <n-select
            :value="settingsStore.settings.check_interval"
            :options="intervalOptions"
            style="width: 140px;"
            @update:value="handleCheckIntervalChange"
          />
        </div>

        <div class="setting-item">
          <div class="setting-copy">
            <div class="setting-title">Token 定期保活</div>
            <div class="setting-description">启用后仅刷新 OAuth 账号的 accessToken 和 refreshToken，成功后写回本机加密库。</div>
          </div>
          <n-switch
            :value="settingsStore.settings.token_keepalive_enabled === 'true'"
            @update:value="handleTokenKeepaliveChange"
          />
        </div>
      </div>
    </section>

    <section class="surface-panel">
      <div class="settings-section-head">
        <div>
          <h2 class="panel-heading">关于</h2>
        </div>
      </div>

      <div class="about-header">
        <div class="about-logo">
          <svg width="38" height="38" viewBox="0 0 24 24" fill="none">
            <path
              d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </div>
        <div>
          <div class="about-title">CodexX</div>
          <div class="about-version">版本 {{ appVersion }}</div>
        </div>
      </div>

      <div class="setting-list">
        <div class="setting-item">
          <div class="setting-copy">
            <div class="setting-title">自动更新</div>
            <div class="setting-description">启动时检测新版本，确认后在线下载、安装并重启。</div>
          </div>
          <n-switch
            :value="settingsStore.settings.auto_update_enabled === 'true'"
            @update:value="handleAutoUpdateChange"
          />
        </div>

        <div class="setting-item">
          <div class="setting-copy">
            <div class="setting-title">GitHub</div>
            <div class="setting-description github-address">{{ githubRepositoryUrl }}</div>
          </div>
          <n-button secondary @click="handleOpenGitHub">
            打开仓库
          </n-button>
        </div>

        <div class="setting-item">
          <div class="setting-copy">
            <div class="setting-title">检查更新</div>
          </div>
          <n-button secondary :loading="checkingUpdate" @click="handleCheckUpdate">
            检查更新
          </n-button>
        </div>

        <div class="setting-item">
          <div class="setting-copy">
            <div class="setting-title">更新日志</div>
          </div>
          <n-button secondary @click="showCurrentChangelogDialog">
            查看日志
          </n-button>
        </div>
      </div>
    </section>

    <section class="surface-panel">
      <div class="settings-section-head">
        <div>
          <h2 class="panel-heading">安全</h2>
        </div>
      </div>

      <div class="security-note">
        <div class="security-note-icon">
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
            <path
              d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"
              stroke="currentColor"
              stroke-width="1.6"
            />
          </svg>
        </div>
        <div>
          <div class="security-note-title">主密钥</div>
          <div class="security-note-copy">重置后需要重新导入所有凭证。</div>
        </div>
      </div>

      <div class="setting-list">
        <div class="setting-item">
          <div class="setting-copy">
            <div class="setting-title">重新生成主密钥</div>
            <div class="setting-description">会使现有凭证失效。</div>
          </div>
          <n-button type="error" secondary @click="handleRegenKey">重新生成</n-button>
        </div>
      </div>
    </section>

    <section class="surface-panel">
      <div class="settings-section-head">
        <div>
          <h2 class="panel-heading">危险操作</h2>
        </div>
      </div>

      <n-alert type="error" :show-icon="false" style="margin-top: 14px;">
        清除后将删除全部账号历史用量。
      </n-alert>

      <div class="setting-list">
        <div class="setting-item">
          <div class="setting-copy">
            <div class="setting-title">清除所有用量数据</div>
            <div class="setting-description">只清除本机历史记录。</div>
          </div>
          <n-button type="error" secondary @click="handleClearUsage">清除数据</n-button>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, h, onBeforeUnmount, onMounted, ref } from 'vue'
import { useDialog, useMessage } from 'naive-ui'
import packageJson from '../../package.json'
import changelogMarkdown from '../../CHANGELOG.md?raw'
import { usageService } from '@/services'
import { useSettingsStore } from '@/stores/settings'
import type { AppSettings } from '@/types'
import { checkAppUpdate, installAppUpdate } from '@/utils/app-updater'
import { renderMarkdownLite } from '@/utils/markdown-lite'
import {
  extractLatestChangelogSections,
  normalizeUpdateChangelogBody,
} from '@/utils/update-changelog'

const message = useMessage()
const dialog = useDialog()
const settingsStore = useSettingsStore()
const appVersion = packageJson.version
const githubRepositoryUrl = 'https://github.com/weimin96/CodexX'
const githubReleasesUrl = `${githubRepositoryUrl}/releases`

const autosaveState = ref<'idle' | 'saving' | 'saved' | 'error'>('idle')
const checkingUpdate = ref(false)
let autosaveResetTimer: ReturnType<typeof setTimeout> | null = null
const recentChangelog = computed(() => extractLatestChangelogSections(changelogMarkdown, 3))

const intervalOptions = [
  { label: '1 分钟', value: '60' },
  { label: '5 分钟', value: '300' },
  { label: '15 分钟', value: '900' },
  { label: '30 分钟', value: '1800' },
  { label: '1 小时', value: '3600' },
]

const autosaveStatusLabel = computed(() => {
  switch (autosaveState.value) {
    case 'saving':
      return '保存中'
    case 'saved':
      return '已保存'
    case 'error':
      return '保存失败'
    default:
      return '自动保存'
  }
})

onMounted(async () => {
  try {
    await settingsStore.loadSettings()
  } catch (error) {
    console.warn('读取设置失败', error)
    message.error('读取设置失败')
  }
})

onBeforeUnmount(() => {
  clearAutosaveResetTimer()
})

async function handleThemeChange(value: string | number | boolean | null) {
  if (value !== 'light' && value !== 'dark') {
    return
  }

  if (value === settingsStore.settings.theme) {
    return
  }

  await persistSettingsChange({ theme: value }, '主题保存失败')
}

async function handleCheckIntervalChange(value: string | null) {
  if (!value) {
    return
  }

  if (value === settingsStore.settings.check_interval) {
    return
  }

  await persistSettingsChange({ check_interval: value }, '检测间隔保存失败')
}

async function handleAutostartChange(enabled: boolean) {
  if (String(enabled) === settingsStore.settings.autostart) {
    return
  }

  autosaveState.value = 'saving'
  clearAutosaveResetTimer()

  try {
    await settingsStore.setAutostart(enabled)
    markAutosaveSuccess()
  } catch (error) {
    console.warn('开机自启保存失败', error)
    autosaveState.value = 'error'
    message.error(enabled ? '开启开机自启失败' : '关闭开机自启失败')
  }
}

async function handleTokenKeepaliveChange(enabled: boolean) {
  if (String(enabled) === settingsStore.settings.token_keepalive_enabled) {
    return
  }

  await persistSettingsChange(
    { token_keepalive_enabled: String(enabled) },
    enabled ? '开启 Token 保活失败' : '关闭 Token 保活失败',
  )
}

async function handleAutoUpdateChange(enabled: boolean) {
  if (String(enabled) === settingsStore.settings.auto_update_enabled) {
    return
  }

  await persistSettingsChange(
    { auto_update_enabled: String(enabled) },
    enabled ? '开启自动更新失败' : '关闭自动更新失败',
  )
}

async function persistSettingsChange(
  updates: Partial<AppSettings>,
  failureMessage: string,
) {
  autosaveState.value = 'saving'
  clearAutosaveResetTimer()

  try {
    await settingsStore.saveSettings(updates)
    markAutosaveSuccess()
  } catch (error) {
    console.warn('设置保存失败', error)
    autosaveState.value = 'error'
    message.error(failureMessage)
  }
}

function markAutosaveSuccess() {
  clearAutosaveResetTimer()
  autosaveState.value = 'saved'
  autosaveResetTimer = setTimeout(() => {
    autosaveState.value = 'idle'
    autosaveResetTimer = null
  }, 1600)
}

function clearAutosaveResetTimer() {
  if (!autosaveResetTimer) {
    return
  }

  clearTimeout(autosaveResetTimer)
  autosaveResetTimer = null
}

function handleRegenKey() {
  dialog.error({
    title: '警告',
    content:
      '重新生成主密钥后，所有已存储的凭证将无法解密，账号需要重新录入凭证。确定继续？',
    positiveText: '确认重新生成',
    negativeText: '取消',
    onPositiveClick: () => {
      message.info('请在系统凭据库中手动删除 CodexX 使用的 codex-manager 条目后重启应用')
    },
  })
}

function handleClearUsage() {
  dialog.warning({
    title: '清除用量数据',
    content: '此操作将删除所有账号的历史用量记录，且不可恢复。确定清除？',
    positiveText: '确认清除',
    negativeText: '取消',
    onPositiveClick: async () => {
      try {
        await usageService.clearUsageData()
        message.success('用量数据已清除')
      } catch (error) {
        console.warn('清除用量数据失败', error)
        message.error('清除用量数据失败')
      }
    },
  })
}

function showCurrentChangelogDialog() {
  showRecentChangelogDialog(recentChangelog.value)
}

async function handleOpenGitHub() {
  await openExternalLink(githubRepositoryUrl, '打开 GitHub 地址失败')
}

async function handleOpenGitHubReleases() {
  await openExternalLink(githubReleasesUrl, '打开 GitHub Releases 失败')
}

async function openExternalLink(url: string, failureLog: string) {
  try {
    const { open } = await import('@tauri-apps/plugin-shell')
    await open(url)
  } catch (error) {
    console.warn(failureLog, error)
    window.open(url, '_blank', 'noopener,noreferrer')
  }
}

async function handleCheckUpdate() {
  checkingUpdate.value = true
  try {
    const outcome = await checkAppUpdate()
    if (outcome.status === 'unsupported') {
      message.warning('检查更新功能仅在 Tauri 应用中可用')
    } else if (outcome.status === 'not_available') {
      message.success('当前已是最新版本')
    } else if (outcome.status === 'available') {
      showUpdateInstallDialog(outcome.version, outcome.body)
    }
  } catch (error) {
    console.warn('检查更新失败', error)
    message.error('检查更新失败，请稍后再试')
  } finally {
    checkingUpdate.value = false
  }
}

function showUpdateInstallDialog(version: string, body?: string) {
  dialog.info({
    title: `发现新版本 ${version}`,
    content: () => renderChangelogDialogContent(body),
    positiveText: '下载并重启',
    negativeText: '稍后处理',
    onPositiveClick: async () => {
      checkingUpdate.value = true
      try {
        const outcome = await installAppUpdate()
        if (outcome.status === 'not_available') {
          message.success('当前已是最新版本')
        }
      } catch (error) {
        console.warn('安装更新失败', error)
        message.error('安装更新失败，请稍后再试')
      } finally {
        checkingUpdate.value = false
      }
    },
  })
}

function showRecentChangelogDialog(body?: string) {
  dialog.info({
    title: '更新日志（最近三个版本）',
    content: () => renderChangelogDialogContent(body),
    positiveText: '知道了',
    negativeText: '查看更多日志',
    onNegativeClick: () => {
      void handleOpenGitHubReleases()
    },
  })
}

function renderChangelogDialogContent(body?: string) {
  const markdownBody = normalizeUpdateChangelogBody(body)
  return h(
    'div',
    {
      class: 'changelog-dialog-content',
    },
    [
      renderMarkdownLite(markdownBody, (url) => {
        void openExternalLink(url, '打开外部链接失败')
      }),
    ],
  )
}
</script>

<style scoped>
.settings-page {
  gap: 14px;
}

.settings-section-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.setting-list {
  margin-top: 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.setting-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding-bottom: 14px;
  border-bottom: 1px solid var(--app-border);
}

.setting-item:last-child {
  padding-bottom: 0;
  border-bottom: none;
}

.setting-copy {
  flex: 1;
  min-width: 0;
}

.setting-title {
  font-size: 15px;
  line-height: 1.3;
  letter-spacing: -0.12px;
  font-weight: 600;
}

.setting-description {
  margin-top: 4px;
  font-size: 13px;
  line-height: 1.43;
  letter-spacing: -0.12px;
  color: var(--app-ink-secondary);
}

.autosave-pill {
  display: inline-flex;
  align-items: center;
  min-height: 28px;
  padding: 0 12px;
  border-radius: var(--app-radius-control);
  background: var(--app-surface-muted);
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-secondary);
  white-space: nowrap;
}

.autosave-pill-saving {
  color: var(--app-blue);
}

.autosave-pill-saved {
  background: var(--status-normal-soft);
  color: var(--status-normal);
}

.autosave-pill-error {
  background: var(--status-error-soft);
  color: var(--status-error);
}

.security-note {
  margin-top: 14px;
  padding: 16px;
  border-radius: 18px;
  display: flex;
  align-items: flex-start;
  gap: 12px;
  background: var(--app-surface-muted);
}

.security-note-icon {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 113, 227, 0.12);
  color: var(--app-blue);
  flex-shrink: 0;
}

.security-note-title {
  font-size: 15px;
  line-height: 1.3;
  letter-spacing: -0.12px;
  font-weight: 600;
}

.security-note-copy {
  margin-top: 4px;
  font-size: 13px;
  line-height: 1.43;
  letter-spacing: -0.12px;
  color: var(--app-ink-secondary);
}

.about-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 14px;
}

.about-logo {
  width: 56px;
  height: 56px;
  border-radius: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--app-black);
  color: #ffffff;
}

.about-title {
  font-family: var(--font-display);
  font-size: 18px;
  line-height: 1.2;
  letter-spacing: 0.12px;
  font-weight: 600;
}

.about-version {
  margin-top: 2px;
  font-size: 13px;
  line-height: 1.43;
  color: var(--app-ink-secondary);
}

.github-address {
  word-break: break-all;
}

.changelog-dialog-content {
  max-height: 360px;
  overflow: auto;
  margin: 0;
  font-family: var(--font-sans);
  font-size: 13px;
  line-height: 1.6;
  color: var(--app-ink);
}

.changelog-dialog-content :deep(.markdown-lite-h1) {
  font-size: 15px;
  margin: 0 0 10px 0;
  font-weight: 700;
}

.changelog-dialog-content :deep(.markdown-lite-h2) {
  font-size: 14px;
  margin: 14px 0 8px 0;
  font-weight: 700;
}

.changelog-dialog-content :deep(.markdown-lite-h3) {
  font-size: 13px;
  margin: 12px 0 6px 0;
  font-weight: 700;
  color: var(--app-ink-secondary);
}

.changelog-dialog-content :deep(.markdown-lite-p) {
  margin: 8px 0;
  white-space: pre-wrap;
  word-break: break-word;
}

.changelog-dialog-content :deep(.markdown-lite-ul) {
  margin: 8px 0;
  padding-left: 18px;
}

.changelog-dialog-content :deep(.markdown-lite-li) {
  margin: 4px 0;
  white-space: pre-wrap;
  word-break: break-word;
}

.changelog-dialog-content :deep(.markdown-lite-inline-code) {
  padding: 1px 6px;
  border-radius: 8px;
  background: var(--app-surface-muted);
  font-family: var(--font-mono);
  font-size: 12px;
}

.changelog-dialog-content :deep(.markdown-lite-fence) {
  margin: 10px 0;
  padding: 10px 12px;
  border-radius: 14px;
  background: var(--app-surface-muted);
}

.changelog-dialog-content :deep(.markdown-lite-pre) {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
}

.changelog-dialog-content :deep(.markdown-lite-code) {
  font-family: var(--font-mono);
  font-size: 12px;
}

.changelog-dialog-content :deep(.markdown-lite-link) {
  color: var(--app-blue);
  text-decoration: none;
}

.changelog-dialog-content :deep(.markdown-lite-link:hover) {
  text-decoration: underline;
}

@media (max-width: 640px) {
  .settings-section-head,
  .setting-item {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
