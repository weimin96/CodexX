<template>
  <div class="app-page settings-page">
    <section class="surface-panel">
      <div class="settings-section-head">
        <div>
          <h1 class="panel-heading">通用</h1>
          <p class="panel-copy">修改后自动保存。</p>
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

        <div class="setting-item">
          <div class="setting-copy">
            <div class="setting-title">状态检测间隔</div>
            <div class="setting-description">后台检查频率。</div>
          </div>
          <n-select
            :value="settingsStore.settings.check_interval"
            :options="intervalOptions"
            style="width: 140px;"
            @update:value="handleCheckIntervalChange"
          />
        </div>
      </div>
    </section>

    <section class="surface-panel">
      <div class="settings-section-head">
        <div>
          <h2 class="panel-heading">安全</h2>
          <p class="panel-copy">凭证仅保存在本机加密库。</p>
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
          <h2 class="panel-heading">关于</h2>
          <p class="panel-copy">版本与更新。</p>
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
          <div class="about-title">Codex Manager</div>
          <div class="about-version">版本 0.1.0</div>
        </div>
      </div>

      <div class="setting-list">
        <div class="setting-item">
          <div class="setting-copy">
            <div class="setting-title">检查更新</div>
            <div class="setting-description">使用 Tauri updater。</div>
          </div>
          <n-button secondary :loading="checkingUpdate" @click="handleCheckUpdate">
            检查更新
          </n-button>
        </div>
      </div>
    </section>

    <section class="surface-panel">
      <div class="settings-section-head">
        <div>
          <h2 class="panel-heading">危险操作</h2>
          <p class="panel-copy">以下操作不可恢复。</p>
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
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { isTauri as detectTauriRuntime } from '@tauri-apps/api/core'
import { useDialog, useMessage } from 'naive-ui'
import { useSettingsStore } from '@/stores/settings'
import type { AppSettings } from '@/types'

const isTauri = detectTauriRuntime()
let checkForUpdates: (() => Promise<any>) | null = null

if (isTauri) {
  import('@tauri-apps/plugin-updater').then((module) => {
    checkForUpdates = module.check
  })
}

const message = useMessage()
const dialog = useDialog()
const settingsStore = useSettingsStore()

const autosaveState = ref<'idle' | 'saving' | 'saved' | 'error'>('idle')
const checkingUpdate = ref(false)
let autosaveResetTimer: ReturnType<typeof setTimeout> | null = null

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
      message.info('请在系统凭据库中手动删除 codex-manager 条目后重启应用')
    },
  })
}

function handleClearUsage() {
  dialog.warning({
    title: '清除用量数据',
    content: '此操作将删除所有账号的历史用量记录，且不可恢复。确定清除？',
    positiveText: '确认清除',
    negativeText: '取消',
    onPositiveClick: () => {
      message.success('用量数据已清除')
    },
  })
}

async function handleCheckUpdate() {
  if (!isTauri) {
    message.warning('检查更新功能仅在 Tauri 应用中可用')
    return
  }

  checkingUpdate.value = true
  try {
    if (!checkForUpdates) {
      message.error('更新功能未初始化')
      return
    }

    const update = await checkForUpdates()
    if (update) {
      message.info(`发现新版本 ${update.version}，正在下载...`)
      await update.downloadAndInstall()
    } else {
      message.success('当前已是最新版本')
    }
  } catch (error) {
    console.warn('检查更新失败', error)
    message.error('检查更新失败，请稍后再试')
  } finally {
    checkingUpdate.value = false
  }
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

@media (max-width: 640px) {
  .settings-section-head,
  .setting-item {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
