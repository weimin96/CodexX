<template>
  <div v-if="account" class="app-page">
    <n-breadcrumb class="page-breadcrumb">
      <n-breadcrumb-item @click="router.push('/accounts')" style="cursor: pointer;">
        账号列表
      </n-breadcrumb-item>
      <n-breadcrumb-item>{{ account.name }}</n-breadcrumb-item>
    </n-breadcrumb>

    <section class="page-hero page-hero-light">
      <div class="page-hero-copy">
        <span class="page-eyebrow">账号</span>
        <h1 class="page-title">{{ account.name }}</h1>
        <p class="page-subtitle">
          {{ AUTH_TYPE_LABELS[account.auth_type] }}
          <template v-if="account.email"> · {{ account.email }}</template>
          <template v-if="account.organization"> · {{ account.organization }}</template>
        </p>
        <div class="page-hero-actions">
          <n-button secondary :loading="checking" @click="handleCheck">检测状态</n-button>
          <n-button
            v-if="!account.is_default"
            type="primary"
            @click="handleSetDefault"
          >
            设为默认
          </n-button>
          <n-button secondary @click="showEditModal = true">编辑账号</n-button>
          <n-button secondary :loading="refreshing" @click="handleRefreshToken">
            刷新 Token
          </n-button>
        </div>
      </div>

      <div class="hero-detail-panel">
        <div class="hero-avatar" :style="{ background: account.color }">
          {{ account.avatar_text ?? account.name[0]?.toUpperCase() }}
        </div>
        <div class="hero-status">
          <StatusDot :status="account.status" show-label />
        </div>
        <div class="hero-pill-list">
          <span v-if="account.is_default" class="hero-pill hero-pill-dark">默认账号</span>
          <span v-if="account.codex_plan_type" class="hero-pill hero-pill-blue">
            {{ formatPlanType(account.codex_plan_type) }}
          </span>
          <span class="hero-pill">创建于 {{ formatDate(account.created_at) }}</span>
        </div>
        <div v-if="account.status_message" class="status-message" :class="account.status">
          {{ account.status_message }}
        </div>
      </div>
    </section>

    <section class="two-column-grid">
      <div class="surface-panel">
        <h2 class="panel-heading">账号信息</h2>
        <p class="panel-copy">基础信息。</p>
        <div class="data-pair-list detail-list">
          <div class="data-pair">
            <span class="data-pair-label">账号 ID</span>
            <span class="data-pair-value account-id">{{ account.id }}</span>
          </div>
          <div class="data-pair">
            <span class="data-pair-label">认证方式</span>
            <span class="data-pair-value">{{ AUTH_TYPE_LABELS[account.auth_type] }}</span>
          </div>
          <div class="data-pair">
            <span class="data-pair-label">状态</span>
            <span class="data-pair-value">
              <StatusDot :status="account.status" show-label />
            </span>
          </div>
          <div class="data-pair">
            <span class="data-pair-label">最后更新</span>
            <span class="data-pair-value">{{ formatDate(account.updated_at) }}</span>
          </div>
          <div class="data-pair">
            <span class="data-pair-label">最后检测</span>
            <span class="data-pair-value">
              {{ account.last_checked_at ? formatDate(account.last_checked_at) : '从未检测' }}
            </span>
          </div>
        </div>
      </div>

      <div class="surface-panel surface-panel-dark">
        <h2 class="panel-heading">用量快览</h2>
        <p class="panel-copy">本月汇总。</p>
        <div v-if="usageLoading" class="panel-loading">
          <n-spin size="small" />
        </div>
        <div v-else-if="summary" class="metric-grid usage-metrics">
          <div class="metric-card">
            <span class="metric-label">输入 Token</span>
            <strong class="metric-value">{{ formatTokens(summary.total_input_tokens) }}</strong>
          </div>
          <div class="metric-card">
            <span class="metric-label">输出 Token</span>
            <strong class="metric-value">{{ formatTokens(summary.total_output_tokens) }}</strong>
          </div>
          <div class="metric-card">
            <span class="metric-label">请求次数</span>
            <strong class="metric-value">{{ summary.total_requests }}</strong>
          </div>
          <div class="metric-card">
            <span class="metric-label">费用估算</span>
            <strong class="metric-value">${{ summary.total_cost.toFixed(4) }}</strong>
          </div>
        </div>
        <div v-else class="usage-empty">
          <p>当前还没有可展示的用量统计。</p>
        </div>
        <button class="detail-link" type="button" @click="goToUsage">查看详细统计</button>
      </div>
    </section>

    <section class="surface-panel">
      <h2 class="panel-heading">认证管理</h2>
      <p class="panel-copy">查看凭证状态与刷新结果。</p>
      <div class="auth-grid">
        <div class="auth-credential-card">
          <span class="auth-label">凭证遮罩</span>
          <strong class="auth-value">{{ '•'.repeat(32) }}</strong>
        </div>
        <div class="auth-credential-card">
          <span class="auth-label">刷新操作</span>
          <n-button secondary :loading="refreshing" @click="handleRefreshToken">
            刷新 Token
          </n-button>
        </div>
      </div>
      <n-alert
        v-if="authResult"
        :type="authAlertType"
        :title="authResult.message"
        style="margin-top: 18px;"
      />
    </section>

    <n-modal v-model:show="showEditModal" preset="card" title="编辑账号" style="width: 520px;">
      <n-form :model="editForm" label-placement="top">
        <n-form-item label="账号名称">
          <n-input v-model:value="editForm.name" />
        </n-form-item>
        <n-form-item label="邮箱">
          <n-input v-model:value="editForm.email" />
        </n-form-item>
        <n-form-item label="组织">
          <n-input v-model:value="editForm.organization" />
        </n-form-item>
        <n-form-item label="更新凭证（留空保持不变）">
          <n-input
            v-model:value="editForm.credential_value"
            type="password"
            show-password-on="click"
            placeholder="留空则不修改凭证"
          />
        </n-form-item>
        <n-form-item label="标识颜色">
          <div class="color-row">
            <button
              v-for="color in PRESET_COLORS"
              :key="color"
              type="button"
              class="color-dot"
              :class="{ selected: editForm.color === color }"
              :style="{ background: color }"
              @click="editForm.color = color"
            />
          </div>
        </n-form-item>
        <div class="modal-footer">
          <n-button secondary @click="showEditModal = false">取消</n-button>
          <n-button type="primary" :loading="editLoading" @click="handleEdit">保存</n-button>
        </div>
      </n-form>
    </n-modal>
  </div>

  <div v-else class="app-page">
    <section class="surface-panel empty-panel">
      <n-spin />
      <p>正在加载账号详情。</p>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { useAccountStore } from '@/stores/account'
import { useUsageStore } from '@/stores/usage'
import { authService } from '@/services'
import { AUTH_TYPE_LABELS } from '@/types'
import type { AuthCheckResult } from '@/types'
import StatusDot from '@/components/common/StatusDot.vue'
import { format, parseISO } from 'date-fns'

const route = useRoute()
const router = useRouter()
const message = useMessage()
const accountStore = useAccountStore()
const usageStore = useUsageStore()

const accountId = computed(() => route.params.id as string)
const account = computed(() => accountStore.accounts.find((item) => item.id === accountId.value))

const checking = computed(() => accountStore.checkingStatus.has(accountId.value))
const usageLoading = ref(false)
const summary = computed(() => usageStore.getSummary(accountId.value, 'month'))

const showEditModal = ref(false)
const editLoading = ref(false)
const refreshing = ref(false)
const authResult = ref<AuthCheckResult | null>(null)

const authAlertType = computed(() => {
  if (!authResult.value) return 'default'
  return {
    valid: 'success',
    expired: 'warning',
    invalid: 'error',
    unknown: 'default',
  }[authResult.value.status] as 'success' | 'warning' | 'error' | 'default'
})

const PRESET_COLORS = [
  '#0071e3',
  '#1f8f5f',
  '#b26a00',
  '#c4314b',
  '#7254d1',
  '#0f9fb0',
  '#d96a20',
  '#b53b70',
  '#147d68',
  '#64748b',
]

const editForm = ref({
  name: '',
  email: '',
  organization: '',
  color: '#0071e3',
  credential_value: '',
})

onMounted(async () => {
  if (account.value) {
    editForm.value = {
      name: account.value.name,
      email: account.value.email ?? '',
      organization: account.value.organization ?? '',
      color: account.value.color,
      credential_value: '',
    }
  }

  usageLoading.value = true
  try {
    await usageStore.loadUsage(accountId.value, 'month')
  } finally {
    usageLoading.value = false
  }
})

function formatDate(iso: string) {
  try {
    return format(parseISO(iso), 'yyyy-MM-dd HH:mm')
  } catch {
    return iso
  }
}

function formatTokens(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`
  return String(value)
}

function formatPlanType(planType: string): string {
  const normalized = planType.trim()
  return normalized ? normalized.toUpperCase() : '未知计划'
}

async function handleCheck() {
  await accountStore.checkAccountStatus(accountId.value)
  message.success('状态检测完成')
}

async function handleSetDefault() {
  await accountStore.switchAccount(accountId.value)
  message.success('已设为默认账号')
}

async function handleRefreshToken() {
  refreshing.value = true
  try {
    authResult.value = await authService.refreshToken(accountId.value)
  } catch {
    message.error('刷新失败')
  } finally {
    refreshing.value = false
  }
}

async function handleEdit() {
  editLoading.value = true
  try {
    await accountStore.updateAccount({
      id: accountId.value,
      name: editForm.value.name,
      email: editForm.value.email || undefined,
      organization: editForm.value.organization || undefined,
      color: editForm.value.color,
      credential_value: editForm.value.credential_value || undefined,
    })
    showEditModal.value = false
    message.success('账号已更新')
  } finally {
    editLoading.value = false
  }
}

function goToUsage() {
  accountStore.setActive(accountId.value)
  router.push('/usage')
}
</script>

<style scoped>
.page-breadcrumb {
  margin-bottom: -8px;
}

.hero-detail-panel {
  width: min(280px, 100%);
  display: flex;
  flex-direction: column;
  gap: 10px;
  align-items: flex-start;
}

.hero-avatar {
  width: 56px;
  height: 56px;
  border-radius: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-display);
  font-size: 22px;
  font-weight: 600;
  color: #ffffff;
}

.hero-status {
  display: flex;
  align-items: center;
}

.hero-pill-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.hero-pill {
  display: inline-flex;
  align-items: center;
  min-height: 28px;
  padding: 0 12px;
  border-radius: var(--app-radius-control);
  background: var(--app-surface-muted);
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-secondary);
}

.hero-pill-dark {
  background: var(--app-ink);
  color: #ffffff;
}

.hero-pill-blue {
  background: rgba(0, 113, 227, 0.12);
  color: var(--app-blue);
}

.detail-list .data-pair-value {
  display: flex;
  justify-content: flex-end;
}

.account-id {
  word-break: break-all;
}

.panel-loading,
.usage-empty {
  min-height: 120px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.usage-empty p {
  margin: 0;
  color: rgba(255, 255, 255, 0.72);
}

.usage-metrics {
  margin-top: 14px;
}

.detail-link {
  margin-top: 12px;
  border: none;
  background: transparent;
  color: var(--app-link-dark);
  padding: 0;
  font-size: 13px;
  line-height: 1.43;
  letter-spacing: -0.12px;
  cursor: pointer;
}

.auth-grid {
  margin-top: 14px;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.auth-credential-card {
  padding: 14px 16px;
  border-radius: 18px;
  background: var(--app-surface-muted);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.auth-label {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
}

.auth-value {
  font-family: var(--font-display);
  font-size: 16px;
  line-height: 1.2;
  letter-spacing: 0.12px;
}

.color-row {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.color-dot {
  width: 28px;
  height: 28px;
  border: 2px solid transparent;
  border-radius: 50%;
  cursor: pointer;
  transition:
    transform 0.18s ease,
    box-shadow 0.18s ease;
}

.color-dot:hover {
  transform: scale(1.08);
}

.color-dot.selected {
  border-color: #ffffff;
  box-shadow: 0 0 0 2px rgba(29, 29, 31, 0.16);
}

.modal-footer {
  margin-top: 14px;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

@media (max-width: 768px) {
  .auth-grid {
    grid-template-columns: 1fr;
  }
}
</style>
