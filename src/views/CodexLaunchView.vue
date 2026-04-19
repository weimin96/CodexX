<template>
  <div class="app-page codex-page">
    <section class="surface-panel section-grid">
      <div class="toolbar-header">
        <div class="toolbar-copy">
          <h2 class="panel-heading">启动方式</h2>
          <p class="panel-supporting">
            CLI 会直接打开 Codex 命令窗口。App 会启动系统安装的 Codex 应用。
          </p>
        </div>
        <div class="codex-actions">
          <n-button
            type="primary"
            :loading="launchingCli"
            @click="handleLaunchCodexCli"
          >
            启动 CLI
          </n-button>
          <n-button
            secondary
            :loading="launchingApp"
            @click="handleLaunchCodexApp"
          >
            启动 App
          </n-button>
        </div>
      </div>

      <div class="codex-launch-grid">
        <div class="control-grid">
          <div class="control-block">
            <span class="control-label">模型</span>
            <n-select
              v-model:value="codexModel"
              :options="modelOptions"
              clearable
              filterable
              placeholder="读取配置默认模型"
            />
          </div>
        </div>

        <div class="directory-row">
          <div class="control-block directory-input">
            <span class="control-label">工作目录</span>
            <n-input v-model:value="codexWorkingDirectory" placeholder="选择文件夹或输入路径" />
          </div>
          <n-button secondary class="directory-button" @click="handleChooseDirectory">
            选择文件夹
          </n-button>
        </div>

        <div v-if="projectHistory.length > 0" class="control-block">
          <span class="control-label">最近项目</span>
          <div class="project-history">
            <button
              v-for="projectPath in projectHistory"
              :key="projectPath"
              class="project-chip"
              type="button"
              @click="useProjectPath(projectPath)"
            >
              {{ projectPath }}
            </button>
          </div>
        </div>
      </div>

      <n-alert
        v-if="lastCodexResult"
        :type="lastCodexResult.status === 'failed' ? 'warning' : 'success'"
        :show-icon="false"
      >
        {{ lastCodexResult.message }}
      </n-alert>
    </section>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { isTauri as detectTauriRuntime } from '@tauri-apps/api/core'
import { useMessage } from 'naive-ui'
import { usageService } from '@/services'
import type { CodexLaunchResult, CodexModelOption } from '@/types'
import { useAccountStore } from '@/stores/account'

const PROJECT_HISTORY_KEY = 'codex-manager.codex-project-history'
const MAX_PROJECT_HISTORY = 8

const isTauri = detectTauriRuntime()
const message = useMessage()
const accountStore = useAccountStore()

const codexWorkingDirectory = ref('')
const codexModel = ref<string | null>(null)
const launchingCli = ref(false)
const launchingApp = ref(false)
const lastCodexResult = ref<CodexLaunchResult | null>(null)
const projectHistory = ref<string[]>([])
const modelOptions = ref<CodexModelOption[]>([])

onMounted(async () => {
  if (accountStore.accounts.length === 0) {
    await accountStore.loadAccounts()
  }
  projectHistory.value = readProjectHistory()
  await loadLauncherConfig()
})

async function handleChooseDirectory() {
  if (!isTauri) {
    message.warning('请选择 Tauri 应用中的本地文件夹')
    return
  }

  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selectedPath = await open({
      directory: true,
      multiple: false,
    })

    if (typeof selectedPath === 'string') {
      codexWorkingDirectory.value = selectedPath
      rememberProjectPath(selectedPath)
    }
  } catch (error) {
    console.warn('选择工作目录失败', error)
    message.error('选择工作目录失败')
  }
}

async function handleLaunchCodexCli() {
  launchingCli.value = true
  try {
    const workingDirectory = normalizeOptionalText(codexWorkingDirectory.value)
    const result = await usageService.launchCodexCli({
      account_id: accountStore.activeAccount?.id,
      working_directory: workingDirectory,
      model: normalizeOptionalText(codexModel.value),
    })
    if (workingDirectory) rememberProjectPath(workingDirectory)
    lastCodexResult.value = result
    message.success(result.message)
  } catch (error) {
    console.warn('启动 Codex CLI 失败', error)
    message.error('启动 Codex CLI 失败')
  } finally {
    launchingCli.value = false
  }
}

async function handleLaunchCodexApp() {
  launchingApp.value = true
  try {
    const result = await usageService.launchCodexApp({
      account_id: accountStore.activeAccount?.id,
    })
    lastCodexResult.value = result
    message.success(result.message)
  } catch (error) {
    console.warn('启动 Codex App 失败', error)
    message.error('启动 Codex App 失败')
  } finally {
    launchingApp.value = false
  }
}

async function loadLauncherConfig() {
  try {
    const launcherConfig = await usageService.getCodexLauncherConfig()
    modelOptions.value = launcherConfig.model_options

    if (!normalizeOptionalText(codexModel.value) && launcherConfig.default_model) {
      codexModel.value = launcherConfig.default_model
    }
  } catch (error) {
    console.warn('读取 Codex 启动配置失败', error)
    message.warning('读取 Codex 默认模型失败，已回退为手动选择')
  }
}

function useProjectPath(projectPath: string) {
  codexWorkingDirectory.value = projectPath
  rememberProjectPath(projectPath)
}

function rememberProjectPath(projectPath: string) {
  const normalizedPath = projectPath.trim()
  if (!normalizedPath) return

  const nextHistory = [
    normalizedPath,
    ...projectHistory.value.filter((item) => item !== normalizedPath),
  ].slice(0, MAX_PROJECT_HISTORY)
  projectHistory.value = nextHistory
  localStorage.setItem(PROJECT_HISTORY_KEY, JSON.stringify(nextHistory))
}

function readProjectHistory(): string[] {
  try {
    const rawValue = localStorage.getItem(PROJECT_HISTORY_KEY)
    if (!rawValue) return []
    const parsed = JSON.parse(rawValue)
    if (!Array.isArray(parsed)) return []
    return parsed
      .filter((item): item is string => typeof item === 'string' && item.trim().length > 0)
      .slice(0, MAX_PROJECT_HISTORY)
  } catch {
    return []
  }
}

function normalizeOptionalText(value: string | null | undefined): string | undefined {
  const trimmed = value?.trim() ?? ''
  return trimmed ? trimmed : undefined
}
</script>

<style scoped>
.codex-page {
  gap: 14px;
}

.toolbar-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 14px;
}

.toolbar-copy {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.codex-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.codex-launch-grid {
  display: grid;
  gap: 14px;
}

.control-grid {
  display: grid;
  grid-template-columns: minmax(0, 280px);
  gap: 12px;
}

.control-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 0;
}

.control-label {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
}

.directory-row {
  display: flex;
  align-items: flex-end;
  gap: 10px;
}

.directory-input {
  flex: 1;
}

.directory-button {
  flex-shrink: 0;
}

.project-history {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.project-chip {
  max-width: 100%;
  border: 1px solid var(--app-border);
  border-radius: var(--app-radius-control);
  padding: 7px 10px;
  background: var(--app-surface-muted);
  color: var(--app-ink-secondary);
  font-size: 12px;
  line-height: 1.33;
  text-align: left;
  cursor: pointer;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.project-chip:hover {
  color: var(--app-blue);
  border-color: rgba(0, 113, 227, 0.36);
}

@media (max-width: 960px) {
  .toolbar-header,
  .directory-row {
    align-items: stretch;
    flex-direction: column;
  }

  .control-grid {
    grid-template-columns: 1fr;
  }
}
</style>
