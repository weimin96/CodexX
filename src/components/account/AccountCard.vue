<template>
  <div class="account-card" :class="{ default: account.is_default }">
    <div class="card-top">
      <div class="card-profile">
        <div class="avatar" :style="{ background: account.color }">
          {{ displayAvatarText }}
        </div>
        <div class="card-copy">
          <div class="card-name-row">
            <h3 class="card-name">{{ displayName }}</h3>
            <StatusDot
              :status="statusDisplay.tone"
              :label="statusDisplay.label"
              :title="statusDisplay.title"
            />
            <span v-if="account.is_default" class="label-pill label-pill-contrast">默认</span>
            <span v-if="account.codex_plan_type" class="label-pill label-pill-blue">
              {{ formatPlanType(account.codex_plan_type) }}
            </span>
          </div>
          <div v-if="displayEmail" class="card-subtitle">{{ displayEmail }}</div>
          <div v-else class="card-subtitle">{{ AUTH_TYPE_LABELS[account.auth_type] }}</div>
        </div>
      </div>
      <div class="card-side">
        <div class="card-actions" @click.stop>
          <n-tooltip trigger="hover">
            <template #trigger>
              <n-button
                circle
                secondary
                size="small"
                class="card-icon-button"
                @click="$emit('detail')"
              >
                <template #icon>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                    <path
                      d="M9 6h11M9 12h11M9 18h11M4 6h.01M4 12h.01M4 18h.01"
                      stroke="currentColor"
                      stroke-width="1.8"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    />
                  </svg>
                </template>
              </n-button>
            </template>
            查看详情
          </n-tooltip>

          <n-tooltip trigger="hover">
            <template #trigger>
              <n-button
                circle
                secondary
                size="small"
                class="card-icon-button"
                :loading="checking"
                @click="$emit('check')"
              >
                <template #icon>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                    <path
                      d="M20 12a8 8 0 1 1-2.34-5.66"
                      stroke="currentColor"
                      stroke-width="1.8"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    />
                    <path
                      d="M20 4v6h-6"
                      stroke="currentColor"
                      stroke-width="1.8"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    />
                  </svg>
                </template>
              </n-button>
            </template>
            检测状态
          </n-tooltip>

          <n-tooltip v-if="!account.is_default" trigger="hover">
            <template #trigger>
              <n-button
                circle
                secondary
                size="small"
                class="card-icon-button"
                @click="$emit('set-default')"
              >
                <template #icon>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                    <path
                      d="M12 3.5l2.63 5.33 5.87.85-4.25 4.14 1 5.84L12 17.1l-5.25 2.76 1-5.84-4.25-4.14 5.87-.85L12 3.5z"
                      stroke="currentColor"
                      stroke-width="1.7"
                      stroke-linejoin="round"
                    />
                  </svg>
                </template>
              </n-button>
            </template>
            设为默认
          </n-tooltip>

          <n-tooltip trigger="hover">
            <template #trigger>
              <n-button
                circle
                secondary
                size="small"
                type="error"
                class="card-icon-button"
                @click="$emit('delete')"
              >
                <template #icon>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                    <path
                      d="M4 7h16M9 11v6M15 11v6M10 4h4l1 2H9l1-2zm-3 3 1 12h8l1-12"
                      stroke="currentColor"
                      stroke-width="1.8"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    />
                  </svg>
                </template>
              </n-button>
            </template>
            删除账号
          </n-tooltip>
        </div>
      </div>
    </div>

    <div v-if="hasCodexUsage(account)" class="usage-panel">
      <AccountQuotaChart
        :five-hour="account.codex_usage_5h"
        :one-week="account.codex_usage_week"
        :featured="account.is_default"
      />
    </div>

    <div v-if="account.codex_usage_error" class="status-message warning">
      账号资料暂不可用：{{ displayUsageError }}
    </div>

    <div v-if="displayStatusMessage" class="status-message" :class="statusDisplay.tone">
      {{ displayStatusMessage }}
    </div>

    <div v-if="statusDiagnostic" class="status-diagnostic">
      {{ statusDiagnostic }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { AUTH_TYPE_LABELS } from '@/types'
import type { Account } from '@/types'
import StatusDot from '@/components/common/StatusDot.vue'
import AccountQuotaChart from '@/components/account/AccountQuotaChart.vue'
import { computed } from 'vue'
import {
  resolveAccountAvatarText,
  resolveAccountDisplayName,
} from '@/utils/account-display'
import {
  resolveAccountStatusDiagnostic,
  resolveAccountStatusDisplay,
  resolveAccountStatusMessage,
} from '@/utils/account-status'

const props = defineProps<{
  account: Account
  checking?: boolean
}>()

defineEmits<{
  detail: []
  check: []
  'set-default': []
  delete: []
}>()

const displayName = computed(() => resolveAccountDisplayName(props.account))
const displayAvatarText = computed(() => resolveAccountAvatarText(props.account))
const displayEmail = computed(() => props.account.email?.trim() || '')
const displayUsageError = computed(() => formatUsageError(props.account.codex_usage_error))
const statusDisplay = computed(() => resolveAccountStatusDisplay(props.account))
const displayStatusMessage = computed(() => resolveAccountStatusMessage(props.account))
const statusDiagnostic = computed(() => resolveAccountStatusDiagnostic(props.account))

function hasCodexUsage(account: Account): boolean {
  return Boolean(account.codex_usage_5h || account.codex_usage_week)
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
  transition:
    box-shadow 0.2s ease;
}

.account-card:hover {
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

.card-side {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
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
  word-break: break-all;
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

.usage-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.card-actions {
  display: flex;
  gap: 8px;
}

.card-icon-button {
  flex-shrink: 0;
}

.status-diagnostic {
  margin-top: -6px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--app-ink-tertiary);
  word-break: break-word;
}

.account-card.default .status-diagnostic {
  color: var(--app-feature-ink-tertiary);
}

@media (max-width: 640px) {
  .account-card {
    padding: 16px;
  }
}
</style>
