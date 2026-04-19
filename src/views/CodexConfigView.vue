<template>
  <div class="app-page codex-config-page">
    <section class="surface-panel section-grid">
      <div class="toolbar-header">
        <div class="toolbar-copy">
          <h2 class="panel-heading">Codex 配置</h2>
        </div>
        <div class="config-actions">
          <n-button secondary :loading="loading" @click="loadConfig">重新加载</n-button>
        </div>
      </div>

      <div class="config-meta-grid">
        <div class="config-meta-item">
          <span>文件路径</span>
          <strong>{{ configPath }}</strong>
        </div>
        <div class="config-meta-item">
          <span>文件状态</span>
          <strong>{{ snapshot?.exists ? '已存在' : '未创建' }}</strong>
        </div>
      </div>

      <n-alert v-if="snapshot?.backup_path" type="success" :show-icon="false">
        已保存，原配置备份到 {{ snapshot.backup_path }}。
      </n-alert>

      <n-alert v-else-if="snapshot && !snapshot.exists" type="info" :show-icon="false">
        当前没有用户级配置文件，保存后会创建新的 config.toml。
      </n-alert>
    </section>

    <section class="surface-panel section-grid">
      <div class="config-form-groups">
        <section
          v-for="group in officialConfigGroups"
          :key="group.title"
          class="config-form-group"
        >
          <div class="config-form-group-head">
            <h3>{{ group.title }}</h3>
            <p>{{ group.description }}</p>
          </div>
          <div class="config-field-list">
            <div v-for="field in group.fields" :key="field.key" class="config-field-row">
              <div class="config-field-title-row">
                <code>{{ field.key }}</code>
                <span>{{ field.type }}</span>
              </div>
              <p>{{ field.description }}</p>

              <n-input
                v-if="isDynamicField(field)"
                :value="dynamicFieldKeys[field.key] ?? ''"
                size="small"
                placeholder="填写实际字段名后保存，例如 mcp_servers.local.command"
                @update:value="(value) => setDynamicFieldKey(field, value)"
              />

              <n-select
                v-if="fieldOptions(field).length > 0"
                :value="selectFieldValue(field)"
                :options="fieldOptions(field)"
                size="small"
                clearable
                :disabled="fieldSavingKey === resolvedFieldKey(field)"
                @update:value="(value) => handleFieldValueChange(field, value)"
              />
              <n-switch
                v-else-if="fieldControlKind(field) === 'boolean'"
                :value="Boolean(fieldValue(field))"
                :disabled="fieldSavingKey === resolvedFieldKey(field)"
                @update:value="(value) => handleFieldValueChange(field, value)"
              />
              <n-input-number
                v-else-if="fieldControlKind(field) === 'number'"
                :value="numberFieldValue(field)"
                size="small"
                :disabled="fieldSavingKey === resolvedFieldKey(field)"
                @update:value="handleNumberFieldUpdate(field, $event)"
                @blur="() => saveField(field)"
              />
              <n-input
                v-else-if="fieldControlKind(field) === 'toml'"
                :value="stringFieldValue(field)"
                type="textarea"
                size="small"
                placeholder="输入 TOML 值，例如 [&quot;AGENTS.md&quot;]"
                :autosize="{ minRows: 2, maxRows: 5 }"
                :disabled="fieldSavingKey === resolvedFieldKey(field)"
                @update:value="(value) => setFieldValue(field, value)"
                @blur="() => saveField(field)"
              />
              <n-input
                v-else
                :value="stringFieldValue(field)"
                size="small"
                placeholder="留空会保存为空字符串"
                :disabled="fieldSavingKey === resolvedFieldKey(field)"
                @update:value="(value) => setFieldValue(field, value)"
                @blur="() => saveField(field)"
                @keyup.enter="() => saveField(field)"
              />

              <div class="config-field-state">
                {{ fieldStateText(field) }}
              </div>
            </div>
          </div>
        </section>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useMessage } from 'naive-ui'
import { codexConfigService } from '@/services'
import type { CodexConfigSnapshot } from '@/types'

interface ConfigReferenceField {
  key: string
  type: string
  description: string
}

interface ConfigReferenceGroup {
  title: string
  description: string
  fields: ConfigReferenceField[]
}

const message = useMessage()
const loading = ref(false)
const snapshot = ref<CodexConfigSnapshot | null>(null)
const fieldValues = ref<Record<string, string | number | boolean | null>>({})
const dynamicFieldKeys = ref<Record<string, string>>({})
const fieldSavingKey = ref<string | null>(null)
const lastSavedFieldKey = ref<string | null>(null)

const configPath = computed(() => snapshot.value?.path ?? '~/.codex/config.toml')

const officialConfigGroups: ConfigReferenceGroup[] = [
  {
    title: '基础与模型',
    description: '控制默认模型、模型上下文、服务层级和通用通信风格。',
    fields: [
      { key: 'model', type: 'string', description: 'Codex 默认使用的模型。' },
      { key: 'review_model', type: 'string', description: '执行 /review 时使用的模型覆盖。' },
      { key: 'model_provider', type: 'string', description: '从 model_providers 中选择模型提供方。' },
      { key: 'model_context_window', type: 'number', description: '当前模型可用的上下文窗口 Token 数。' },
      { key: 'model_auto_compact_token_limit', type: 'number', description: '触发自动压缩历史的 Token 阈值。' },
      { key: 'model_catalog_json', type: 'string(path)', description: '启动时加载的模型目录 JSON 路径。' },
      { key: 'model_instructions_file', type: 'string(path)', description: '替换内置指令的本地文件路径。' },
      { key: 'openai_base_url', type: 'string', description: '内置 OpenAI provider 的 API 基础地址。' },
      { key: 'service_tier', type: 'flex | fast', description: '新回合偏好的服务层级。' },
      { key: 'profile', type: 'string', description: '启动时默认应用的配置 profile。' },
      { key: 'personality', type: 'none | friendly | pragmatic', description: '支持 personality 模型的默认沟通风格。' },
      { key: 'plan_mode_reasoning_effort', type: 'none | minimal | low | medium | high | xhigh', description: 'Plan 模式专用推理强度覆盖。' },
    ],
  },
  {
    title: '审批、沙箱与执行',
    description: '控制命令审批、文件系统沙箱、Windows 沙箱和 shell 环境传递。',
    fields: [
      { key: 'approval_policy', type: 'untrusted | on-request | on-failure | never', description: '控制运行命令前的审批策略。' },
      { key: 'sandbox_mode', type: 'read-only | workspace-write | danger-full-access', description: '控制文件系统与网络沙箱策略。' },
      { key: 'sandbox_workspace_write.writable_roots', type: 'array<string>', description: 'workspace-write 模式额外可写目录。' },
      { key: 'sandbox_workspace_write.network_access', type: 'boolean', description: 'workspace-write 沙箱内是否允许网络访问。' },
      { key: 'sandbox_workspace_write.exclude_tmpdir_env_var', type: 'boolean', description: '是否排除 TMPDIR 指向的临时目录。' },
      { key: 'sandbox_workspace_write.exclude_slash_tmp', type: 'boolean', description: '是否排除 /tmp。' },
      { key: 'windows.sandbox', type: 'unelevated | elevated', description: 'Windows 原生运行时的沙箱模式。' },
      { key: 'windows.sandbox_private_desktop', type: 'boolean', description: '是否在私有桌面运行最终沙箱子进程。' },
      { key: 'shell_environment_policy.exclude', type: 'array<string>', description: '从默认 shell 环境中移除的变量 glob。' },
      { key: 'shell_environment_policy.include_only', type: 'array<string>', description: '只允许传入的环境变量列表。' },
      { key: 'allow_login_shell', type: 'boolean', description: '是否允许以登录 shell 语义启动 shell。' },
      { key: 'hide_agent_reasoning', type: 'boolean', description: '隐藏 TUI 和 codex exec 输出中的 reasoning 事件。' },
      { key: 'file_opener', type: 'vscode | vscode-insiders | windsurf | cursor | none', description: '打开引用位置时使用的编辑器 URI scheme。' },
      { key: 'log_dir', type: 'string(path)', description: 'Codex 写入日志文件的目录。' },
    ],
  },
  {
    title: 'Provider 与外部模型',
    description: '定义自定义模型提供方以及命令式鉴权方式。',
    fields: [
      { key: 'model_providers.<id>', type: 'table', description: '自定义 provider 定义；内置 openai、ollama、lmstudio 不能覆盖。' },
      { key: 'model_providers.<id>.base_url', type: 'string', description: '该 provider 的 API 基础地址。' },
      { key: 'model_providers.<id>.env_http_headers', type: 'map<string,string>', description: '从环境变量填充的 HTTP 头。' },
      { key: 'model_providers.<id>.auth', type: 'table', description: '命令式 bearer token 鉴权配置。' },
      { key: 'model_providers.<id>.auth.command', type: 'string', description: '输出 bearer token 的命令。' },
      { key: 'model_providers.<id>.auth.args', type: 'array<string>', description: '传给 token 命令的参数。' },
      { key: 'model_providers.<id>.auth.cwd', type: 'string(path)', description: 'token 命令工作目录。' },
      { key: 'model_providers.<id>.auth.timeout_ms', type: 'number', description: 'token 命令最长运行时间。' },
      { key: 'model_providers.<id>.auth.refresh_interval_ms', type: 'number', description: '主动刷新 bearer token 的间隔。' },
      { key: 'oss_provider', type: 'lmstudio | ollama', description: '使用 --oss 时选择的 OSS provider。' },
    ],
  },
  {
    title: 'Profile 与项目配置',
    description: '按 profile、项目和工程说明文件控制不同工作区行为。',
    fields: [
      { key: 'profiles.<name>.*', type: 'various', description: 'profile 作用域内的配置覆盖。' },
      { key: 'profiles.<name>.model_catalog_json', type: 'string(path)', description: 'profile 级模型目录 JSON 覆盖。' },
      { key: 'profiles.<name>.model_instructions_file', type: 'string(path)', description: 'profile 级模型指令文件覆盖。' },
      { key: 'profiles.<name>.service_tier', type: 'flex | fast', description: 'profile 级服务层级覆盖。' },
      { key: 'profiles.<name>.web_search', type: 'disabled | cached | live', description: 'profile 级 web search 模式。' },
      { key: 'profiles.<name>.windows.sandbox', type: 'unelevated | elevated', description: 'profile 级 Windows 沙箱模式。' },
      { key: 'projects.<path>.trust_level', type: 'trusted | untrusted', description: '标记项目或 worktree 是否可信。' },
      { key: 'project_doc_fallback_filenames', type: 'array<string>', description: 'AGENTS.md 缺失时尝试读取的备用文件名。' },
      { key: 'project_doc_max_bytes', type: 'number', description: '读取项目说明文件的最大字节数。' },
      { key: 'project_root_markers', type: 'array<string>', description: '向上查找项目根目录时使用的标记文件。' },
    ],
  },
  {
    title: 'MCP 与工具',
    description: '配置 MCP server、OAuth 回调、工具开关和 web search。',
    fields: [
      { key: 'mcp_oauth_callback_port', type: 'integer', description: 'MCP OAuth 本地回调服务固定端口。' },
      { key: 'mcp_oauth_callback_url', type: 'string', description: 'MCP OAuth redirect URI 覆盖。' },
      { key: 'mcp_oauth_credentials_store', type: 'auto | file | keyring', description: 'MCP OAuth 凭据首选存储方式。' },
      { key: 'mcp_servers.<id>.command', type: 'string', description: 'MCP stdio server 启动命令。' },
      { key: 'mcp_servers.<id>.args', type: 'array<string>', description: 'MCP stdio server 启动参数。' },
      { key: 'mcp_servers.<id>.cwd', type: 'string(path)', description: 'MCP stdio server 工作目录。' },
      { key: 'mcp_servers.<id>.url', type: 'string', description: 'MCP streamable HTTP server 地址。' },
      { key: 'mcp_servers.<id>.env', type: 'map<string,string>', description: '转发给 MCP stdio server 的环境变量。' },
      { key: 'mcp_servers.<id>.enabled', type: 'boolean', description: '禁用或启用 MCP server。' },
      { key: 'mcp_servers.<id>.enabled_tools', type: 'array<string>', description: 'MCP 工具允许列表。' },
      { key: 'mcp_servers.<id>.disabled_tools', type: 'array<string>', description: 'MCP 工具拒绝列表。' },
      { key: 'mcp_servers.<id>.required', type: 'boolean', description: '启用后初始化失败会阻止启动或恢复。' },
      { key: 'mcp_servers.<id>.startup_timeout_sec', type: 'number', description: 'MCP server 启动超时时间。' },
      { key: 'mcp_servers.<id>.tool_timeout_sec', type: 'number', description: '单个 MCP 工具调用超时时间。' },
      { key: 'mcp_servers.<id>.oauth_resource', type: 'string', description: 'MCP OAuth 的 RFC 8707 resource 参数。' },
      { key: 'mcp_servers.<id>.scopes', type: 'array<string>', description: 'MCP OAuth 请求的 scope。' },
      { key: 'web_search', type: 'disabled | cached | live', description: 'Codex web search 模式。' },
      { key: 'tools.view_image', type: 'boolean', description: '启用本地图像附件查看工具。' },
      { key: 'tools.web_search', type: 'boolean | table', description: '工具层 web search 配置，可含 context_size、allowed_domains 和 location。' },
    ],
  },
  {
    title: '功能开关、记忆与历史',
    description: '控制实验功能、会话历史和 Codex memory 行为。',
    fields: [
      { key: 'features.agents', type: 'boolean', description: '启用多 agent 协作工具。' },
      { key: 'features.personality', type: 'boolean', description: '启用 personality 选择控件。' },
      { key: 'features.prevent_idle_sleep', type: 'boolean', description: '运行回合时阻止机器睡眠。' },
      { key: 'features.shell_snapshot', type: 'boolean', description: '快照 shell 环境以加快重复命令。' },
      { key: 'features.shell_tool', type: 'boolean', description: '启用默认 shell 工具。' },
      { key: 'features.skill_mcp_dependency_install', type: 'boolean', description: '允许提示并安装技能缺失的 MCP 依赖。' },
      { key: 'features.undo', type: 'boolean', description: '启用 undo 支持。' },
      { key: 'features.unified_exec', type: 'boolean', description: '启用统一 PTY exec 工具。' },
      { key: 'features.web_search', type: 'boolean', description: '旧 web search 开关，官方建议使用顶层 web_search。' },
      { key: 'history.max_bytes', type: 'number', description: 'history.jsonl 最大字节数。' },
      { key: 'history.persistence', type: 'save-all | none', description: '是否保存会话历史。' },
      { key: 'memories.use_memories', type: 'boolean', description: '是否向未来会话注入既有 memories。' },
      { key: 'memories.generate_memories', type: 'boolean', description: '是否从新建线程生成 memory 输入。' },
      { key: 'memories.extract_model', type: 'string', description: '逐线程 memory 提取模型覆盖。' },
      { key: 'memories.consolidation_model', type: 'string', description: '全局 memory 合并模型覆盖。' },
      { key: 'memories.max_rollout_age_days', type: 'number', description: '参与 memory 生成的线程最大天数。' },
      { key: 'memories.max_rollouts_per_startup', type: 'number', description: '每次启动处理的 rollout 候选数量。' },
      { key: 'memories.max_raw_memories_for_consolidation', type: 'number', description: '合并时保留的最近 raw memories 数量。' },
      { key: 'memories.max_unused_days', type: 'number', description: 'memory 未使用超过该天数后不再参与合并。' },
      { key: 'memories.min_rollout_idle_hours', type: 'number', description: '线程进入 memory 生成前需要空闲的小时数。' },
      { key: 'memories.no_memories_if_mcp_or_web_search', type: 'boolean', description: '使用 MCP 或 web search 的线程不生成 memory。' },
    ],
  },
  {
    title: '界面、通知与管理',
    description: '控制 TUI、Windows 引导状态、登录限制和 agent 定义。',
    fields: [
      { key: 'tui.alternate_screen', type: 'auto | always | never', description: 'TUI 是否使用 alternate screen。' },
      { key: 'tui.animations', type: 'boolean', description: '是否启用终端动画。' },
      { key: 'tui.notification_method', type: 'auto | osc9 | bel', description: '终端未聚焦时的通知方式。' },
      { key: 'tui.notifications', type: 'boolean | array<string>', description: '启用 TUI 通知，可限制事件类型。' },
      { key: 'tui.show_tooltips', type: 'boolean', description: '是否显示 TUI 欢迎页提示。' },
      { key: 'tui.status_line', type: 'array<string> | null', description: 'TUI 底部状态栏项目列表。' },
      { key: 'tui.terminal_title', type: 'array<string> | null', description: '终端标题项目列表。' },
      { key: 'tui.theme', type: 'string', description: '语法高亮主题名。' },
      { key: 'tui.model_availability_nux.<model>', type: 'integer', description: '按模型记录的内部启动提示状态。' },
      { key: 'windows_wsl_setup_acknowledged', type: 'boolean', description: 'Windows WSL 引导确认状态。' },
      { key: 'forced_login_method', type: 'chatgpt | api', description: '限制允许的登录方式。' },
      { key: 'forced_chatgpt_workspace_id', type: 'string(uuid)', description: '限制 ChatGPT 登录到指定 workspace。' },
      { key: 'feedback.enabled', type: 'boolean', description: '是否允许 /feedback 反馈提交。' },
      { key: 'agents.<name>.description', type: 'string', description: 'Codex 选择和派生该 agent 类型时使用的说明。' },
      { key: 'agents.<name>.config_file', type: 'string(path)', description: '该 agent 角色加载的 TOML 配置层。' },
      { key: 'agents.<name>.nickname_candidates', type: 'array<string>', description: '派生 agent 可选昵称池。' },
      { key: 'agents.max_threads', type: 'number', description: '允许同时打开的 agent 线程数量。' },
      { key: 'agents.max_depth', type: 'number', description: '允许的派生 agent 嵌套深度。' },
      { key: 'agents.job_max_runtime_seconds', type: 'number', description: 'spawn_agents_on_csv 任务默认运行超时。' },
    ],
  },
]

onMounted(() => {
  void loadConfig()
})

async function loadConfig() {
  loading.value = true
  try {
    const nextSnapshot = await codexConfigService.readConfig()
    applySnapshot(nextSnapshot)
  } catch (error) {
    console.warn('读取 Codex 配置失败', error)
    message.error(formatError(error, '读取 Codex 配置失败'))
  } finally {
    loading.value = false
  }
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

function applySnapshot(nextSnapshot: CodexConfigSnapshot) {
  snapshot.value = nextSnapshot
  const nextValues: Record<string, string | number | boolean | null> = {}
  const entryMap = new Map(nextSnapshot.parsed_entries.map((entry) => [entry.key, entry]))

  for (const group of officialConfigGroups) {
    for (const field of group.fields) {
      if (isDynamicField(field)) {
        if (!(field.key in fieldValues.value)) {
          nextValues[field.key] = defaultFieldValue(field)
        } else {
          nextValues[field.key] = fieldValues.value[field.key]
        }
        continue
      }

      const entry = entryMap.get(field.key)
      nextValues[field.key] = entry ? normalizeEntryValue(field, entry.value) : defaultFieldValue(field)
    }
  }

  fieldValues.value = nextValues
}

function isDynamicField(field: ConfigReferenceField): boolean {
  return /[<>*]/.test(field.key)
}

function resolvedFieldKey(field: ConfigReferenceField): string {
  if (!isDynamicField(field)) {
    return field.key
  }

  return dynamicFieldKeys.value[field.key]?.trim() || field.key
}

function setDynamicFieldKey(field: ConfigReferenceField, value: string) {
  dynamicFieldKeys.value = {
    ...dynamicFieldKeys.value,
    [field.key]: value,
  }
}

function fieldControlKind(field: ConfigReferenceField): 'string' | 'number' | 'boolean' | 'toml' {
  if (fieldOptions(field).length > 0) {
    return 'string'
  }

  if (
    field.type.includes('array') ||
    field.type.includes('map') ||
    field.type.includes('table') ||
    field.type.includes('various') ||
    field.type.includes('null')
  ) {
    return 'toml'
  }

  if (field.type.includes('boolean')) {
    return 'boolean'
  }

  if (field.type.includes('number') || field.type.includes('integer')) {
    return 'number'
  }

  return 'string'
}

function fieldOptions(field: ConfigReferenceField) {
  if (!field.type.includes('|')) {
    return []
  }

  const options = field.type
    .split('|')
    .map((option) => option.trim())
    .filter(Boolean)

  if (
    options.some((option) =>
      ['array', 'table', 'map', 'boolean', 'null', 'various'].some((complexType) =>
        option.includes(complexType),
      ),
    )
  ) {
    return []
  }

  return options.map((option) => ({
      label: option,
      value: option,
    }))
}

function fieldValue(field: ConfigReferenceField) {
  return fieldValues.value[field.key] ?? defaultFieldValue(field)
}

function stringFieldValue(field: ConfigReferenceField): string {
  const value = fieldValue(field)
  return typeof value === 'string' ? value : value == null ? '' : String(value)
}

function selectFieldValue(field: ConfigReferenceField): string | null {
  const value = fieldValue(field)
  return typeof value === 'string' && value ? value : null
}

function numberFieldValue(field: ConfigReferenceField): number | null {
  const value = fieldValue(field)
  if (typeof value === 'number') {
    return value
  }

  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

function handleNumberFieldUpdate(field: ConfigReferenceField, value: number | null) {
  setFieldValue(field, value)
}

function setFieldValue(field: ConfigReferenceField, value: string | number | boolean | null) {
  fieldValues.value = {
    ...fieldValues.value,
    [field.key]: value,
  }
}

async function handleFieldValueChange(
  field: ConfigReferenceField,
  value: string | number | boolean | null,
) {
  setFieldValue(field, value)
  await saveField(field)
}

async function saveField(field: ConfigReferenceField) {
  const key = resolvedFieldKey(field)
  if (isDynamicField(field) && key === field.key) {
    message.warning('请先填写实际字段名')
    return
  }

  const value = buildTomlValue(field, fieldValue(field))
  if (!value) {
    message.warning('请填写有效配置值')
    return
  }

  fieldSavingKey.value = key
  try {
    const nextSnapshot = await codexConfigService.saveConfigField({ key, value })
    applySnapshot(nextSnapshot)
    lastSavedFieldKey.value = key
    message.success(`已保存 ${key}`)
  } catch (error) {
    console.warn('保存 Codex 配置字段失败', error)
    message.error(formatError(error, `保存 ${key} 失败`))
  } finally {
    fieldSavingKey.value = null
  }
}

function buildTomlValue(
  field: ConfigReferenceField,
  value: string | number | boolean | null,
): string | null {
  const kind = fieldControlKind(field)
  if (kind === 'boolean') {
    return Boolean(value) ? 'true' : 'false'
  }

  if (kind === 'number') {
    const parsed = typeof value === 'number' ? value : Number(value)
    return Number.isFinite(parsed) ? String(parsed) : null
  }

  if (kind === 'toml') {
    const text = String(value ?? '').trim()
    return text || null
  }

  return JSON.stringify(String(value ?? ''))
}

function normalizeEntryValue(field: ConfigReferenceField, value: string) {
  if (fieldControlKind(field) === 'boolean') {
    return value === 'true'
  }

  if (fieldControlKind(field) === 'number') {
    const parsed = Number(value)
    return Number.isFinite(parsed) ? parsed : null
  }

  return value
}

function defaultFieldValue(field: ConfigReferenceField): string | number | boolean | null {
  const kind = fieldControlKind(field)
  if (kind === 'boolean') return false
  if (kind === 'number') return null
  return ''
}

function fieldStateText(field: ConfigReferenceField): string {
  const key = resolvedFieldKey(field)
  if (fieldSavingKey.value === key) {
    return '保存中'
  }

  if (lastSavedFieldKey.value === key) {
    return '已保存'
  }

  return isDynamicField(field) ? '动态字段需填写实际字段名' : '修改后自动保存该字段'
}
</script>

<style scoped>
.codex-config-page {
  gap: 14px;
}

.toolbar-header,
.panel-inline-head {
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

.config-actions,
.doc-links {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  flex-wrap: wrap;
}

.doc-links a {
  color: var(--app-blue);
  font-size: 12px;
  line-height: 1.33;
  text-decoration: none;
}

.config-meta-grid {
  display: grid;
  grid-template-columns: minmax(0, 2fr) minmax(120px, 0.6fr);
  gap: 10px;
}

.config-meta-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
  padding: 12px;
  border-radius: 12px;
  background: var(--app-surface-muted);
}

.config-meta-item span {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
}

.config-meta-item strong {
  min-width: 0;
  font-size: 13px;
  line-height: 1.35;
  color: var(--app-ink);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.codex-config-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.15fr) minmax(320px, 0.85fr);
  gap: 14px;
  align-items: start;
}

.editor-panel,
.parsed-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
  min-width: 0;
}

.dirty-pill {
  display: inline-flex;
  align-items: center;
  min-height: 28px;
  padding: 0 10px;
  border-radius: var(--app-radius-control);
  background: var(--app-surface-muted);
  color: var(--app-ink-secondary);
  font-size: 11px;
  line-height: 1.33;
  white-space: nowrap;
}

.dirty-pill.active {
  color: var(--app-blue);
}

.config-editor :deep(textarea) {
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
  font-size: 12px;
  line-height: 1.55;
}

.config-form-groups {
  display: grid;
  gap: 16px;
}

.config-form-group {
  display: grid;
  gap: 10px;
}

.config-form-group-head {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.config-form-group-head h3 {
  margin: 0;
  font-size: 15px;
  line-height: 1.3;
  color: var(--app-ink);
}

.config-form-group-head p {
  margin: 0;
  font-size: 12px;
  line-height: 1.43;
  color: var(--app-ink-secondary);
}

.config-field-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.config-field-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
  padding: 10px;
  border: 1px solid var(--app-border);
  border-radius: 12px;
  background: var(--app-surface-muted);
}

.config-field-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-width: 0;
}

.config-field-title-row code {
  min-width: 0;
  color: var(--app-ink);
  font-size: 12px;
  line-height: 1.35;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.config-field-title-row span {
  flex-shrink: 0;
  color: var(--app-ink-tertiary);
  font-size: 11px;
  line-height: 1.33;
}

.config-field-row p {
  margin: 0;
  color: var(--app-ink-secondary);
  font-size: 12px;
  line-height: 1.43;
}

.config-field-state {
  min-height: 16px;
  color: var(--app-ink-tertiary);
  font-size: 11px;
  line-height: 1.33;
}

@media (max-width: 1100px) {
  .codex-config-grid,
  .config-field-list {
    grid-template-columns: 1fr;
  }

  .config-meta-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 640px) {
  .toolbar-header,
  .panel-inline-head {
    align-items: stretch;
    flex-direction: column;
  }

  .config-actions,
  .doc-links {
    justify-content: flex-start;
  }
}
</style>
