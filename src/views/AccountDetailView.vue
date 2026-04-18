<template>
  <div class="view-container" v-if="account">
    <!-- Breadcrumb -->
    <n-breadcrumb>
      <n-breadcrumb-item @click="router.push('/accounts')" style="cursor:pointer">账号列表</n-breadcrumb-item>
      <n-breadcrumb-item>{{ account.name }}</n-breadcrumb-item>
    </n-breadcrumb>

    <!-- Header card -->
    <div class="detail-hero">
      <div class="hero-avatar" :style="{ background: account.color }">
        {{ account.avatar_text ?? account.name[0]?.toUpperCase() }}
      </div>
      <div class="hero-info">
        <div class="hero-name">
          {{ account.name }}
          <n-tag v-if="account.is_default" type="info" size="small">默认</n-tag>
        </div>
        <div class="hero-meta">
          <span>{{ AUTH_TYPE_LABELS[account.auth_type] }}</span>
          <template v-if="account.email">
            <span class="dot">·</span>
            <span>{{ account.email }}</span>
          </template>
          <template v-if="account.organization">
            <span class="dot">·</span>
            <span>{{ account.organization }}</span>
          </template>
        </div>
        <div class="hero-status">
          <StatusDot :status="account.status" show-label />
          <span class="status-msg" v-if="account.status_message">{{ account.status_message }}</span>
        </div>
      </div>
      <div class="hero-actions">
        <n-button
          :loading="checking"
          size="small"
          @click="handleCheck"
        >检测状态</n-button>
        <n-button
          v-if="!account.is_default"
          size="small"
          type="primary"
          ghost
          @click="handleSetDefault"
        >设为默认</n-button>
        <n-button size="small" @click="showEditModal = true">编辑</n-button>
      </div>
    </div>

    <!-- Details grid -->
    <div class="detail-grid">
      <!-- Account Info -->
      <n-card title="账号信息" size="small">
        <n-descriptions :column="1" label-placement="left" size="small">
          <n-descriptions-item label="账号 ID">
            <n-text code>{{ account.id }}</n-text>
          </n-descriptions-item>
          <n-descriptions-item label="认证方式">{{ AUTH_TYPE_LABELS[account.auth_type] }}</n-descriptions-item>
          <n-descriptions-item label="创建时间">{{ formatDate(account.created_at) }}</n-descriptions-item>
          <n-descriptions-item label="最后更新">{{ formatDate(account.updated_at) }}</n-descriptions-item>
          <n-descriptions-item label="最后检测">{{ account.last_checked_at ? formatDate(account.last_checked_at) : '从未' }}</n-descriptions-item>
          <n-descriptions-item label="状态">
            <StatusDot :status="account.status" show-label />
          </n-descriptions-item>
        </n-descriptions>
      </n-card>

      <!-- Quick Usage -->
      <n-card title="用量快览（本月）" size="small">
        <div v-if="usageLoading" class="usage-loading">
          <n-spin size="small" />
        </div>
        <div v-else-if="summary" class="usage-quick">
          <div class="usage-stat">
            <div class="stat-val">{{ formatTokens(summary.total_input_tokens) }}</div>
            <div class="stat-key">输入 Token</div>
          </div>
          <div class="usage-stat">
            <div class="stat-val">{{ formatTokens(summary.total_output_tokens) }}</div>
            <div class="stat-key">输出 Token</div>
          </div>
          <div class="usage-stat">
            <div class="stat-val">{{ summary.total_requests }}</div>
            <div class="stat-key">请求次数</div>
          </div>
          <div class="usage-stat">
            <div class="stat-val">${{ summary.total_cost.toFixed(4) }}</div>
            <div class="stat-key">费用估算</div>
          </div>
        </div>
        <div class="card-link" @click="goToUsage">查看详细统计 →</div>
      </n-card>
    </div>

    <!-- Auth section -->
    <n-card title="认证管理" size="small">
      <div class="auth-section">
        <div class="credential-row">
          <n-text code class="credential-masked">{{ '•'.repeat(32) }}</n-text>
          <n-button size="tiny" quaternary @click="handleRefreshToken" :loading="refreshing">
            刷新 Token
          </n-button>
        </div>
        <n-alert v-if="authResult" :type="authAlertType" :title="authResult.message" style="margin-top:12px;" />
      </div>
    </n-card>

    <!-- Edit Modal -->
    <n-modal v-model:show="showEditModal" preset="card" title="编辑账号" style="width: 480px;">
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
          <n-input v-model:value="editForm.credential_value" type="password" show-password-on="click" placeholder="留空则不修改凭证" />
        </n-form-item>
        <n-form-item label="标识颜色">
          <div class="color-row">
            <div
              v-for="c in PRESET_COLORS" :key="c"
              class="color-dot" :class="{ selected: editForm.color === c }"
              :style="{ background: c }" @click="editForm.color = c"
            />
          </div>
        </n-form-item>
        <div class="modal-footer">
          <n-button @click="showEditModal = false">取消</n-button>
          <n-button type="primary" :loading="editLoading" @click="handleEdit">保存</n-button>
        </div>
      </n-form>
    </n-modal>
  </div>

  <div v-else class="loading-state">
    <n-spin />
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
const account = computed(() => accountStore.accounts.find((a) => a.id === accountId.value))

const checking = computed(() => accountStore.checkingStatus.has(accountId.value))
const usageLoading = ref(false)
const summary = computed(() => usageStore.getSummary(accountId.value, 'month'))

const showEditModal = ref(false)
const editLoading = ref(false)
const refreshing = ref(false)
const authResult = ref<AuthCheckResult | null>(null)

const authAlertType = computed(() => {
  if (!authResult.value) return 'default'
  return { valid: 'success', expired: 'warning', invalid: 'error', unknown: 'default' }[authResult.value.status] as any
})

const PRESET_COLORS = ['#4f8ef7', '#18a058', '#f0a020', '#d03050', '#8b5cf6', '#06b6d4', '#f97316', '#ec4899', '#10b981', '#64748b']

const editForm = ref({ name: '', email: '', organization: '', color: '#4f8ef7', credential_value: '' })

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
  try { return format(parseISO(iso), 'yyyy-MM-dd HH:mm') } catch { return iso }
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M'
  if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K'
  return String(n)
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
.view-container { padding: 24px; display: flex; flex-direction: column; gap: 16px; max-width: 900px; }

.detail-hero {
  display: flex;
  align-items: center;
  gap: 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 20px;
}

.hero-avatar {
  width: 56px; height: 56px; border-radius: 14px;
  display: flex; align-items: center; justify-content: center;
  font-size: 24px; font-weight: 700; color: #fff; flex-shrink: 0;
}

.hero-info { flex: 1; min-width: 0; }

.hero-name {
  font-size: 20px; font-weight: 700; color: var(--text-primary);
  display: flex; align-items: center; gap: 8px;
}

.hero-meta {
  font-size: 13px; color: var(--text-secondary); margin-top: 4px;
  display: flex; align-items: center; gap: 4px;
}

.dot { opacity: 0.4; }

.hero-status { display: flex; align-items: center; gap: 8px; margin-top: 6px; }

.status-msg { font-size: 12px; color: var(--text-secondary); }

.hero-actions { display: flex; gap: 8px; flex-shrink: 0; }

.detail-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.usage-quick {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin-bottom: 12px;
}

.usage-stat {
  background: var(--bg-primary);
  border-radius: 8px;
  padding: 12px;
  text-align: center;
}

.stat-val { font-size: 20px; font-weight: 700; color: var(--accent); font-family: 'Fira Code', monospace; }
.stat-key { font-size: 11px; color: var(--text-secondary); margin-top: 2px; }

.usage-loading { display: flex; justify-content: center; padding: 20px 0; }

.card-link {
  font-size: 12px; color: var(--accent); cursor: pointer;
  text-align: right; margin-top: 4px;
}
.card-link:hover { opacity: 0.8; }

.auth-section { display: flex; flex-direction: column; gap: 8px; }

.credential-row { display: flex; align-items: center; gap: 12px; }
.credential-masked { letter-spacing: 2px; opacity: 0.4; }

.color-row { display: flex; gap: 8px; flex-wrap: wrap; }
.color-dot { width: 24px; height: 24px; border-radius: 50%; cursor: pointer; border: 2px solid transparent; transition: all 0.15s; }
.color-dot:hover { transform: scale(1.15); }
.color-dot.selected { border-color: #fff; box-shadow: 0 0 0 2px rgba(255,255,255,0.3); transform: scale(1.15); }

.modal-footer { display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px; }
.loading-state { display: flex; align-items: center; justify-content: center; height: 300px; }
</style>
