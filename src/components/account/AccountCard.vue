<template>
  <div class="account-card" :class="{ default: account.is_default }" @click="$emit('click')">
    <div class="card-top">
      <div class="card-profile">
        <div class="avatar" :style="{ background: account.color }">
          {{ displayAvatarText }}
        </div>
        <div class="card-copy">
          <div class="card-name-row">
            <h3 class="card-name">{{ displayName }}</h3>
            <span v-if="account.is_default" class="label-pill label-pill-contrast">默认</span>
            <span v-if="account.codex_plan_type" class="label-pill label-pill-blue">
              {{ formatPlanType(account.codex_plan_type) }}
            </span>
          </div>
          <div class="card-subtitle">{{ AUTH_TYPE_LABELS[account.auth_type] }}</div>
        </div>
      </div>
      <StatusDot :status="account.status" />
    </div>

    <div class="meta-list">
      <div v-if="account.email" class="meta-item">
        <span class="meta-label">邮箱</span>
        <span class="meta-value">{{ account.email }}</span>
      </div>
      <div v-if="displayOrganization" class="meta-item">
        <span class="meta-label">{{ organizationLabel }}</span>
        <span class="meta-value">{{ displayOrganization }}</span>
      </div>
      <div v-if="account.last_checked_at" class="meta-item">
        <span class="meta-label">最后检测</span>
        <span class="meta-value">{{ formatDate(account.last_checked_at) }}</span>
      </div>
    </div>

    <div v-if="hasCodexUsage(account)" class="usage-grid">
      <div class="usage-card">
        <span class="usage-label">5 小时窗口</span>
        <strong class="usage-value">{{ formatUsageWindow(account.codex_usage_5h) }}</strong>
      </div>
      <div class="usage-card">
        <span class="usage-label">1 周窗口</span>
        <strong class="usage-value">{{ formatUsageWindow(account.codex_usage_week) }}</strong>
      </div>
    </div>

    <div v-if="account.codex_usage_error" class="status-message warning">
      账号资料暂不可用：{{ displayUsageError }}
    </div>

    <div v-if="account.status_message" class="status-message" :class="account.status">
      {{ account.status_message }}
    </div>

    <div class="card-actions" @click.stop>
      <n-button size="small" secondary :loading="checking" @click="$emit('check')">
        检测状态
      </n-button>
      <n-button
        v-if="!account.is_default"
        size="small"
        secondary
        @click="$emit('set-default')"
      >
        设为默认
      </n-button>
      <n-button size="small" secondary type="error" @click="$emit('delete')">
        删除账号
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { AUTH_TYPE_LABELS } from '@/types'
import type { Account, CodexUsageWindow } from '@/types'
import StatusDot from '@/components/common/StatusDot.vue'
import { format, parseISO } from 'date-fns'
import { computed } from 'vue'
import {
  resolveAccountAvatarText,
  resolveAccountDisplayName,
  resolveAccountOrganizationDisplay,
  resolveAccountOrganizationLabel,
} from '@/utils/account-display'

const props = defineProps<{
  account: Account
  checking?: boolean
}>()

defineEmits<{
  click: []
  check: []
  'set-default': []
  delete: []
}>()

const displayName = computed(() => resolveAccountDisplayName(props.account))
const displayAvatarText = computed(() => resolveAccountAvatarText(props.account))
const displayOrganization = computed(() => resolveAccountOrganizationDisplay(props.account))
const organizationLabel = computed(() => resolveAccountOrganizationLabel(props.account) ?? '组织')
const displayUsageError = computed(() => formatUsageError(props.account.codex_usage_error))

function formatDate(iso: string): string {
  try {
    return format(parseISO(iso), 'MM-dd HH:mm')
  } catch {
    return iso
  }
}

function hasCodexUsage(account: Account): boolean {
  return Boolean(account.codex_usage_5h || account.codex_usage_week)
}

function formatUsageWindow(window?: CodexUsageWindow): string {
  if (!window) return '未知'
  return `已用 ${formatPercent(window.used_percent)}`
}

function formatPercent(value: number): string {
  if (!Number.isFinite(value)) return '未知'
  return `${Math.max(0, Math.min(100, value)).toFixed(1)}%`
}

function formatPlanType(planType: string): string {
  const normalized = planType.trim()
  if (!normalized) return '未知计划'
  return normalized.toUpperCase()
}

function formatUsageError(error?: string): string {
  const normalized = error?.trim()
  if (!normalized) return 'Codex 资料接口暂不可用，可稍后重试'

  if (/(^|\D)(401|403)(\D|$)/.test(normalized)) {
    return '登录信息已失效，请重新同步本地账号'
  }

  if (
    normalized.startsWith('Authentication failed:') ||
    normalized.includes('请求 Codex 用量接口失败') ||
    normalized.includes('error sending request') ||
    normalized.includes('SSL')
  ) {
    return 'Codex 资料接口暂不可达，可稍后重试'
  }

  return normalized
}
</script>

<style scoped>
.account-card {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 18px;
  border-radius: 22px;
  background: var(--app-surface);
  box-shadow: var(--app-shadow);
  cursor: pointer;
  transition:
    transform 0.2s ease,
    box-shadow 0.2s ease;
}

.account-card:hover {
  transform: translateY(-2px);
  box-shadow: rgba(0, 0, 0, 0.22) 3px 9px 34px 0px;
}

.account-card.default {
  background: var(--app-feature-surface);
  color: var(--app-feature-ink);
}

.card-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.card-profile {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  min-width: 0;
}

.avatar {
  width: 44px;
  height: 44px;
  border-radius: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: #ffffff;
  font-family: var(--font-display);
  font-size: 16px;
  font-weight: 600;
}

.card-name-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.card-copy {
  min-width: 0;
}

.card-name {
  margin: 0;
  min-width: 0;
  font-family: var(--font-display);
  font-size: 18px;
  line-height: 1.2;
  letter-spacing: 0.12px;
  font-weight: 700;
}

.card-subtitle {
  margin-top: 4px;
  font-size: 13px;
  line-height: 1.43;
  letter-spacing: -0.12px;
  color: var(--app-ink-secondary);
}

.account-card.default .card-subtitle {
  color: var(--app-feature-ink-secondary);
}

.label-pill {
  display: inline-flex;
  align-items: center;
  min-height: 24px;
  padding: 0 10px;
  border-radius: var(--app-radius-control);
  font-size: 11px;
  line-height: 1.33;
  letter-spacing: -0.12px;
}

.label-pill-contrast {
  background: rgba(29, 29, 31, 0.08);
  color: var(--app-ink);
}

.account-card.default .label-pill-contrast {
  background: var(--app-feature-surface-muted);
  color: var(--app-feature-ink);
}

.label-pill-blue {
  background: rgba(0, 113, 227, 0.12);
  color: var(--app-blue);
}

.meta-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.meta-item {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--app-border);
}

.meta-item:last-child {
  padding-bottom: 0;
  border-bottom: none;
}

.account-card.default .meta-item {
  border-bottom-color: var(--app-feature-border);
}

.meta-label {
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
}

.meta-value {
  max-width: 70%;
  text-align: right;
  font-size: 13px;
  line-height: 1.43;
  letter-spacing: -0.12px;
  color: var(--app-ink-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.account-card.default .meta-label,
.account-card.default .meta-value {
  color: var(--app-feature-ink-secondary);
}

.usage-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.usage-card {
  padding: 10px 12px;
  border-radius: 16px;
  background: var(--app-surface-muted);
}

.account-card.default .usage-card {
  background: var(--app-feature-surface-muted);
}

.usage-label {
  display: block;
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-ink-tertiary);
}

.usage-value {
  display: block;
  margin-top: 4px;
  font-family: var(--font-display);
  font-size: 16px;
  line-height: 1.2;
  letter-spacing: 0.12px;
}

.account-card.default .usage-label {
  color: var(--app-feature-ink-tertiary);
}

.card-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

@media (max-width: 640px) {
  .account-card {
    padding: 16px;
  }

  .usage-grid {
    grid-template-columns: 1fr;
  }

  .meta-item {
    flex-direction: column;
    gap: 4px;
  }

  .meta-value {
    max-width: 100%;
    text-align: left;
  }
}
</style>
