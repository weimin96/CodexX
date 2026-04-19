<template>
  <div class="app-page codex-config-page">
    <section v-if="showConfigBanner" class="surface-panel section-grid">
      <n-alert v-if="snapshot?.backup_path" type="success" :show-icon="false">
        已保存，原配置备份到 {{ snapshot.backup_path }}。
      </n-alert>

      <n-alert v-else-if="snapshot && !snapshot.exists" type="info" :show-icon="false">
        当前没有用户级配置文件，保存后会创建新的 config.toml。
      </n-alert>
    </section>

    <section class="surface-panel section-grid config-editor-panel">
      <div class="config-editor-head">
        <div class="config-editor-copy">
          <h2 class="panel-heading">config.toml</h2>
          <p class="config-path">当前文件：{{ configPath }}</p>
        </div>
        <div class="config-actions">
          <n-button secondary :loading="loading" @click="handleReload">
            刷新
          </n-button>
          <n-button
            type="primary"
            :loading="saving"
            :disabled="!isDirty"
            @click="saveConfig"
          >
            保存
          </n-button>
        </div>
      </div>

      <n-input
        v-model:value="rawText"
        class="config-editor"
        type="textarea"
        placeholder="在这里编辑 Codex config.toml 内容"
        :autosize="{ minRows: 24, maxRows: 34 }"
        :disabled="loading || saving"
      />

      <div class="config-editor-footer">
        <span :class="{ active: isDirty }">
          {{ isDirty ? '已修改，等待保存' : '未改动' }}
        </span>
        <span>{{ rawText.length.toLocaleString() }} 字符</span>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useMessage } from 'naive-ui'
import { codexConfigService } from '@/services'
import type { CodexConfigSnapshot } from '@/types'

const message = useMessage()
const loading = ref(false)
const saving = ref(false)
const snapshot = ref<CodexConfigSnapshot | null>(null)
const rawText = ref('')
const savedRawText = ref('')

const configPath = computed(() => snapshot.value?.path ?? '~/.codex/config.toml')
const isDirty = computed(() => rawText.value !== savedRawText.value)
const showConfigBanner = computed(
  () => Boolean(snapshot.value?.backup_path) || Boolean(snapshot.value && !snapshot.value.exists),
)

onMounted(() => {
  void loadConfig()
})

async function handleReload() {
  if (isDirty.value && !window.confirm('当前编辑内容尚未保存，确定重新读取配置文件吗？')) {
    return
  }

  await loadConfig()
}

async function loadConfig() {
  loading.value = true
  try {
    applySnapshot(await codexConfigService.readConfig())
  } catch (error) {
    console.warn('读取 Codex 配置失败', error)
    message.error(formatError(error, '读取 Codex 配置失败'))
  } finally {
    loading.value = false
  }
}

async function saveConfig() {
  saving.value = true
  try {
    applySnapshot(await codexConfigService.saveConfig(rawText.value))
    message.success('已保存 config.toml')
  } catch (error) {
    console.warn('保存 Codex 配置失败', error)
    message.error(formatError(error, '保存 Codex 配置失败'))
  } finally {
    saving.value = false
  }
}

function applySnapshot(nextSnapshot: CodexConfigSnapshot) {
  snapshot.value = nextSnapshot
  rawText.value = nextSnapshot.raw_text
  savedRawText.value = nextSnapshot.raw_text
}

function formatError(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.trim()) {
    return error.message
  }

  if (typeof error === 'string' && error.trim()) {
    return error
  }

  return fallback
}
</script>

<style scoped>
.codex-config-page {
  gap: 14px;
}

.config-editor-panel {
  gap: 14px;
}

.config-editor-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 14px;
}

.config-editor-copy {
  display: grid;
  gap: 6px;
  min-width: 0;
}

.config-path {
  margin: 0;
  color: var(--app-ink-secondary);
  font-size: 12px;
  line-height: 1.43;
  word-break: break-all;
}

.config-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  flex-wrap: wrap;
}

.config-editor :deep(textarea) {
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
  font-size: 12px;
  line-height: 1.55;
}

.config-editor-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  color: var(--app-ink-tertiary);
  font-size: 11px;
  line-height: 1.33;
}

.config-editor-footer .active {
  color: var(--app-blue);
}

@media (max-width: 640px) {
  .config-editor-head,
  .config-editor-footer {
    align-items: stretch;
    flex-direction: column;
  }

  .config-actions {
    justify-content: flex-start;
  }
}
</style>
