<template>
  <div class="account-card" :class="{ default: account.is_default }" @click="$emit('click')">
    <!-- 卡片头部 -->
    <div class="card-header">
      <div class="avatar" :style="{ background: account.color }">
        {{ account.avatar_text ?? account.name[0]?.toUpperCase() }}
      </div>
      <div class="card-info">
        <div class="card-name">
          {{ account.name }}
          <n-tag v-if="account.is_default" size="tiny" type="info" style="margin-left:6px">默认</n-tag>
          <n-tag v-if="account.codex_plan_type" size="tiny" type="success" style="margin-left:6px">
            {{ formatPlanType(account.codex_plan_type) }}
          </n-tag>
        </div>
        <div class="card-sub">{{ AUTH_TYPE_LABELS[account.auth_type] }}</div>
      </div>
      <StatusDot :status="account.status" />
    </div>

    <!-- 元信息 -->
    <div class="card-meta">
      <span v-if="account.email" class="meta-item">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none"><path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z" stroke="currentColor" stroke-width="1.5"/><polyline points="22,6 12,13 2,6" stroke="currentColor" stroke-width="1.5"/></svg>
        {{ account.email }}
      </span>
      <span v-if="account.organization" class="meta-item">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" stroke="currentColor" stroke-width="1.5"/></svg>
        {{ account.organization }}
      </span>
      <span class="meta-item" v-if="account.last_checked_at">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="1.5"/><polyline points="12 6 12 12 16 14" stroke="currentColor" stroke-width="1.5"/></svg>
        {{ formatDate(account.last_checked_at) }}
      </span>
    </div>

    <!-- Codex 用量窗口 -->
    <div v-if="hasCodexUsage(account)" class="usage-window-row">
      <div class="usage-window-pill">
        <span>5 小时</span>
        <strong>{{ formatUsageWindow(account.codex_usage_5h) }}</strong>
      </div>
      <div class="usage-window-pill">
        <span>1 周</span>
        <strong>{{ formatUsageWindow(account.codex_usage_week) }}</strong>
      </div>
    </div>

    <div v-if="account.codex_usage_error" class="card-status-msg warning">
      账号信息同步失败：{{ account.codex_usage_error }}
    </div>

    <!-- 状态消息 -->
    <div v-if="account.status_message" class="card-status-msg" :class="account.status">
      {{ account.status_message }}
    </div>

    <!-- 操作 -->
    <div class="card-actions" @click.stop>
      <n-button
        size="tiny"
        quaternary
        :loading="checking"
        @click="$emit('check')"
        title="检测状态"
      >
        <template #icon>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none"><path d="M23 4v6h-6M1 20v-6h6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
        </template>
        检测
      </n-button>

      <n-button
        v-if="!account.is_default"
        size="tiny"
        quaternary
        @click="$emit('set-default')"
        title="设为默认"
      >
        <template #icon>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none"><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" stroke="currentColor" stroke-width="2"/></svg>
        </template>
        默认
      </n-button>

      <n-button
        size="tiny"
        quaternary
        type="error"
        @click="$emit('delete')"
        title="删除账号"
      >
        <template #icon>
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none"><polyline points="3 6 5 6 21 6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/><path d="M19 6l-1 14H6L5 6M10 11v6M14 11v6M9 6V4h6v2" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
        </template>
        删除
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { AUTH_TYPE_LABELS } from '@/types'
import type { Account, CodexUsageWindow } from '@/types'
import StatusDot from '@/components/common/StatusDot.vue'
import { format, parseISO } from 'date-fns'

defineProps<{
  account: Account
  checking?: boolean
}>()

defineEmits<{
  click: []
  check: []
  'set-default': []
  delete: []
}>()

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
</script>

<style scoped>
.account-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
  cursor: pointer;
  transition: all 0.18s;
  display: flex;
  flex-direction: column;
  gap: 10px;
  position: relative;
  overflow: hidden;
}
.account-card::before {
  content: '';
  position: absolute;
  inset: 0;
  border-radius: 12px;
  background: linear-gradient(135deg, rgba(255,255,255,0.02) 0%, transparent 60%);
  pointer-events: none;
}
.account-card:hover {
  border-color: rgba(79,142,247,0.5);
  box-shadow: 0 0 0 1px rgba(79,142,247,0.15), 0 4px 24px rgba(0,0,0,0.3);
  transform: translateY(-1px);
}
.account-card.default {
  border-color: rgba(79,142,247,0.4);
}
.account-card.default::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: linear-gradient(90deg, #4f8ef7, #7bb3fb);
  border-radius: 12px 12px 0 0;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.avatar {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  font-weight: 700;
  color: #fff;
  flex-shrink: 0;
}

.card-info { flex: 1; min-width: 0; }

.card-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  display: flex;
  align-items: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.card-sub {
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 1px;
}

.card-meta {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.meta-item {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.card-status-msg {
  font-size: 11px;
  padding: 4px 8px;
  border-radius: 6px;
  border-left: 2px solid;
}
.card-status-msg.normal  { background: rgba(24,160,88,0.1);  border-color: #18a058; color: #4ad08a; }
.card-status-msg.warning { background: rgba(240,160,32,0.1); border-color: #f0a020; color: #f0c060; }
.card-status-msg.error   { background: rgba(208,48,80,0.1);  border-color: #d03050; color: #f06080; }
.card-status-msg.expired { background: rgba(139,92,246,0.1); border-color: #8b5cf6; color: #a78bfa; }
.card-status-msg.unknown { background: rgba(144,147,153,0.1); border-color: #909399; color: #b0b3b8; }

.usage-window-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
}

.usage-window-pill {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 7px 8px;
  border: 1px solid rgba(79,142,247,0.24);
  border-radius: 8px;
  background: rgba(79,142,247,0.08);
}

.usage-window-pill span {
  font-size: 10px;
  color: var(--text-secondary);
}

.usage-window-pill strong {
  font-size: 12px;
  color: var(--text-primary);
  font-family: 'Fira Code', monospace;
}

.card-actions {
  display: flex;
  gap: 4px;
  margin-top: 2px;
  border-top: 1px solid var(--border);
  padding-top: 8px;
}
</style>
