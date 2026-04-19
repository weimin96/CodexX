<template>
  <div class="app-page codex-page">
    <section class="page-hero">
      <div class="page-hero-copy">
        <h1 class="page-title">Codex 启动</h1>
      </div>
      <div class="hero-stats">
        <div class="hero-stat">
          <span class="hero-stat-label">账号</span>
          <strong class="hero-stat-value">{{ activeAccountLabel }}</strong>
        </div>
        <div class="hero-stat">
          <span class="hero-stat-label">沙箱</span>
          <strong class="hero-stat-value">{{ sandboxLabel }}</strong>
        </div>
        <div class="hero-stat">
          <span class="hero-stat-label">项目</span>
          <strong class="hero-stat-value">{{ projectHistory.length }}</strong>
        </div>
      </div>
    </section>

    <section class="surface-panel section-grid">
      <div class="toolbar-header">
        <div>
          <h2 class="panel-heading">任务</h2>
        </div>
        <div class="codex-actions">
          <n-button
            secondary
            :disabled="!selectedAccountId"
            :loading="openingCodex"
            @click="handleOpenCodexInteractive"
          >
            打开交互
          </n-button>
          <n-button
            type="primary"
            :disabled="!selectedAccountId"
            :loading="runningCodex"
            @click="handleRunCodexExec"
          >
            运行任务
          </n-button>
        </div>
      </div>

      <div class="codex-launch-grid">
        <div class="control-grid">
          <div class="control-block">
            <span class="control-label">账号</span>
            <n-select
              v-model:value="selectedAccountId"
              :options="accountOptions"
              placeholder="请选择账号"
            />
          </div>
          <div class="control-block">
            <span class="control-label">模型</span>
            <n-input v-model:value="codexModel" placeholder="默认配置" />
          </div>
          <div class="control-block">
            <span class="control-label">沙箱</span>
            <n-select v-model:value="codexSandbox" :options="sandboxOptions" />
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

        <div class="control-block">
          <span class="control-label">任务内容</span>
          <n-input
            v-model:value="codexPrompt"
            type="textarea"
            :autosize="{ minRows: 6, maxRows: 12 }"
            placeholder="输入要交给 Codex 的任务"
          />
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
import { computed, onMounted, ref } from 'vue'
import { isTauri as detectTauriRuntime } from '@tauri-apps/api/core'
import { useMessage } from 'naive-ui'
import { usageService } from '@/services'
import { useAccountStore } from '@/stores/account'
import type { CodexLaunchResult } from '@/types'
import { resolveAccountDisplayName } from '@/utils/account-display'

const PROJECT_HISTORY_KEY = 'codex-manager.codex-project-history'
const MAX_PROJECT_HISTORY = 8

const isTauri = detectTauriRuntime()
const message = useMessage()
const accountStore = useAccountStore()

const selectedAccountId = ref('')
const codexPrompt = ref('')
const codexWorkingDirectory = ref('')
const codexModel = ref('')
const codexSandbox = ref<'read-only' | 'workspace-write' | 'danger-full-access'>('workspace-write')
const runningCodex = ref(false)
const openingCodex = ref(false)
const lastCodexResult = ref<CodexLaunchResult | null>(null)
const projectHistory = ref<string[]>([])

const accountOptions = computed(() =>
  accountStore.accounts.map((account) => ({
    label: resolveAccountDisplayName(account),
    value: account.id,
  })),
)

const activeAccountLabel = computed(() => {
  if (!selectedAccountId.value) return '未选择'
  const account = accountStore.accounts.find((item) => item.id === selectedAccountId.value)
  return account ? resolveAccountDisplayName(account) : '未知账号'
})

const sandboxOptions = [
  { label: '只读', value: 'read-only' },
  { label: '工作区写入', value: 'workspace-write' },
  { label: '完全访问', value: 'danger-full-access' },
]

const sandboxLabel = computed(
  () =>
    sandboxOptions.find((option) => option.value === codexSandbox.value)?.label ?? '工作区写入',
)

onMounted(async () => {
  projectHistory.value = readProjectHistory()

  if (accountStore.accounts.length === 0) {
    await accountStore.loadAccounts()
  }

  const activeAccount = accountStore.activeAccount
  if (activeAccount) {
    selectedAccountId.value = activeAccount.id
  } else if (accountOptions.value.length > 0) {
    selectedAccountId.value = accountOptions.value[0].value
  }
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

async function handleRunCodexExec() {
  if (!selectedAccountId.value) {
    message.warning('请选择账号')
    return
  }

  if (!codexPrompt.value.trim()) {
    message.warning('请输入任务内容')
    return
  }

  runningCodex.value = true
  try {
    const workingDirectory = normalizeOptionalText(codexWorkingDirectory.value)
    const result = await usageService.runCodexExec({
      account_id: selectedAccountId.value,
      prompt: codexPrompt.value.trim(),
      working_directory: workingDirectory,
      model: normalizeOptionalText(codexModel.value),
      sandbox: codexSandbox.value,
    })
    if (workingDirectory) rememberProjectPath(workingDirectory)
    lastCodexResult.value = result
    message.success(result.message)
  } catch (error) {
    console.warn('运行 Codex 任务失败', error)
    message.error('运行 Codex 任务失败')
  } finally {
    runningCodex.value = false
  }
}

async function handleOpenCodexInteractive() {
  if (!selectedAccountId.value) {
    message.warning('请选择账号')
    return
  }

  openingCodex.value = true
  try {
    const workingDirectory = normalizeOptionalText(codexWorkingDirectory.value)
    const result = await usageService.openCodexInteractive({
      account_id: selectedAccountId.value,
      prompt: normalizeOptionalText(codexPrompt.value),
      working_directory: workingDirectory,
      model: normalizeOptionalText(codexModel.value),
      sandbox: codexSandbox.value,
    })
    if (workingDirectory) rememberProjectPath(workingDirectory)
    lastCodexResult.value = result
    message.success(result.message)
  } catch (error) {
    console.warn('打开交互式 Codex 失败', error)
    message.error('打开交互式 Codex 失败')
  } finally {
    openingCodex.value = false
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

function normalizeOptionalText(value: string): string | undefined {
  const trimmed = value.trim()
  return trimmed ? trimmed : undefined
}
</script>

<style scoped>
.codex-page {
  gap: 14px;
}

.toolbar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
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
  grid-template-columns: 1.2fr 0.9fr 0.9fr;
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
