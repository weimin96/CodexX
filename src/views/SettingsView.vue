<template>
  <div class="view-container">
    <div class="view-header">
      <h1 class="view-title">设置</h1>
      <p class="view-sub">配置应用行为和偏好</p>
    </div>

    <div class="settings-grid">
      <!-- General -->
      <n-card title="通用" size="small">
        <div class="setting-group">
          <div class="setting-item">
            <div class="setting-info">
              <div class="setting-label">主题</div>
              <div class="setting-desc">当前仅支持深色模式</div>
            </div>
            <n-select
              v-model:value="localSettings.theme"
              :options="themeOptions"
              size="small"
              style="width: 120px;"
              disabled
            />
          </div>

          <n-divider style="margin: 8px 0;" />

          <div class="setting-item">
            <div class="setting-info">
              <div class="setting-label">开机自启</div>
              <div class="setting-desc">系统启动时自动运行</div>
            </div>
            <n-switch
              :value="localSettings.autostart === 'true'"
              @update:value="onAutostartChange"
            />
          </div>

          <n-divider style="margin: 8px 0;" />

          <div class="setting-item">
            <div class="setting-info">
              <div class="setting-label">状态检测间隔</div>
              <div class="setting-desc">后台自动检测账号状态的频率</div>
            </div>
            <n-select
              v-model:value="localSettings.check_interval"
              :options="intervalOptions"
              size="small"
              style="width: 120px;"
            />
          </div>
        </div>
      </n-card>

      <!-- Security -->
      <n-card title="安全" size="small">
        <div class="setting-group">
          <div class="info-block">
            <div class="info-icon">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" stroke="#18a058" stroke-width="1.5"/></svg>
            </div>
            <div>
              <div class="info-label">加密存储</div>
              <div class="info-desc">所有 API Key 和 Token 通过 AES-256-GCM 加密后存储在本地 SQLite 数据库中，密钥由系统凭据库（keyring）保管，不会以任何形式明文存储。</div>
            </div>
          </div>

          <n-divider style="margin: 12px 0;" />

          <div class="setting-item">
            <div class="setting-info">
              <div class="setting-label">重新生成主密钥</div>
              <div class="setting-desc">警告：重新生成后已有凭证将无法解密</div>
            </div>
            <n-button size="small" type="error" ghost @click="handleRegenKey">
              重新生成
            </n-button>
          </div>
        </div>
      </n-card>

      <!-- About -->
      <n-card title="关于" size="small">
        <div class="setting-group">
          <div class="about-row">
            <div class="app-logo-big">
              <svg width="40" height="40" viewBox="0 0 24 24" fill="none">
                <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" stroke="#4f8ef7" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </div>
            <div>
              <div class="app-name">Codex Manager</div>
              <div class="app-ver">版本 0.1.0</div>
            </div>
          </div>

          <n-divider style="margin: 12px 0;" />

          <div class="tech-stack">
            <div class="tech-title">技术栈</div>
            <div class="tech-tags">
              <n-tag v-for="t in techStack" :key="t" size="small" :bordered="false" style="background: var(--bg-primary);">{{ t }}</n-tag>
            </div>
          </div>

          <n-divider style="margin: 12px 0;" />

          <div class="setting-item">
            <div class="setting-info">
              <div class="setting-label">检查更新</div>
              <div class="setting-desc">当前使用 Tauri updater 自动更新</div>
            </div>
            <n-button size="small" @click="handleCheckUpdate" :loading="checkingUpdate">
              检查更新
            </n-button>
          </div>
        </div>
      </n-card>

      <!-- Danger zone -->
      <n-card title="危险操作" size="small">
        <div class="setting-group">
          <n-alert type="error" :show-icon="false">
            以下操作不可逆，请谨慎操作
          </n-alert>
          <div class="setting-item" style="margin-top: 12px;">
            <div class="setting-info">
              <div class="setting-label">清除所有用量数据</div>
              <div class="setting-desc">删除所有账号的历史用量记录</div>
            </div>
            <n-button size="small" type="error" ghost @click="handleClearUsage">
              清除数据
            </n-button>
          </div>
        </div>
      </n-card>
    </div>

    <!-- Save bar -->
    <div class="save-bar">
      <span class="save-hint">{{ saved ? '✓ 已保存' : '有未保存的更改' }}</span>
      <n-button type="primary" size="small" :loading="saving" @click="handleSave">
        保存设置
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useMessage, useDialog } from 'naive-ui'
import { useSettingsStore } from '@/stores/settings'
import { checkForUpdates } from '@tauri-apps/plugin-updater'

const message = useMessage()
const dialog = useDialog()
const settingsStore = useSettingsStore()

const localSettings = reactive({ ...settingsStore.settings })
const saving = ref(false)
const saved = ref(false)
const checkingUpdate = ref(false)

const themeOptions = [
  { label: '深色', value: 'dark' },
  { label: '浅色', value: 'light' },
]

const intervalOptions = [
  { label: '1 分钟', value: '60' },
  { label: '5 分钟', value: '300' },
  { label: '15 分钟', value: '900' },
  { label: '30 分钟', value: '1800' },
  { label: '1 小时', value: '3600' },
]

const techStack = ['Tauri 2', 'Vue 3', 'TypeScript', 'Rust', 'SQLite', 'AES-256-GCM', 'ECharts', 'Naive UI']

onMounted(async () => {
  await settingsStore.loadSettings()
  Object.assign(localSettings, settingsStore.settings)
})

async function handleSave() {
  saving.value = true
  try {
    await settingsStore.saveSettings({ ...localSettings })
    saved.value = true
    message.success('设置已保存')
    setTimeout(() => { saved.value = false }, 3000)
  } finally {
    saving.value = false
  }
}

async function onAutostartChange(enabled: boolean) {
  localSettings.autostart = String(enabled)
  await settingsStore.setAutostart(enabled)
  message.success(enabled ? '已开启开机自启' : '已关闭开机自启')
}

function handleRegenKey() {
  dialog.error({
    title: '警告',
    content: '重新生成主密钥后，所有已存储的凭证将无法解密，账号需要重新录入凭证。确定继续？',
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
  checkingUpdate.value = true
  try {
    const update = await checkForUpdates()
    if (update) {
      message.info(`发现新版本 ${update.version}，正在下载...`)
      await update.downloadAndInstall()
    } else {
      message.success('当前已是最新版本')
    }
  } catch {
    message.error('检查更新失败，请稍后再试')
  } finally {
    checkingUpdate.value = false
  }
}
</script>

<style scoped>
.view-container { padding: 24px; display: flex; flex-direction: column; gap: 16px; max-width: 800px; }
.view-header { margin-bottom: 4px; }
.view-title { font-size: 22px; font-weight: 700; color: var(--text-primary); }
.view-sub { font-size: 13px; color: var(--text-secondary); margin-top: 2px; }

.settings-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }

.setting-group { display: flex; flex-direction: column; }

.setting-item {
  display: flex; align-items: center; justify-content: space-between;
  gap: 16px; min-height: 48px;
}

.setting-info { flex: 1; min-width: 0; }
.setting-label { font-size: 13px; font-weight: 500; color: var(--text-primary); }
.setting-desc { font-size: 11px; color: var(--text-secondary); margin-top: 2px; }

.info-block {
  display: flex; align-items: flex-start; gap: 12px;
  background: rgba(24,160,88,0.06); border: 1px solid rgba(24,160,88,0.2);
  border-radius: 8px; padding: 12px;
}
.info-icon { flex-shrink: 0; }
.info-label { font-size: 13px; font-weight: 600; color: #4ad08a; }
.info-desc { font-size: 11px; color: var(--text-secondary); margin-top: 4px; line-height: 1.6; }

.about-row { display: flex; align-items: center; gap: 12px; }
.app-name { font-size: 16px; font-weight: 700; color: var(--text-primary); }
.app-ver { font-size: 12px; color: var(--text-secondary); margin-top: 2px; }

.tech-stack { display: flex; flex-direction: column; gap: 8px; }
.tech-title { font-size: 12px; color: var(--text-secondary); }
.tech-tags { display: flex; flex-wrap: wrap; gap: 6px; }

.save-bar {
  display: flex; align-items: center; justify-content: flex-end; gap: 12px;
  padding: 12px 16px; background: var(--bg-secondary);
  border: 1px solid var(--border); border-radius: 10px;
  position: sticky; bottom: 16px;
}
.save-hint { font-size: 12px; color: var(--text-secondary); }
</style>
