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
          </div>
          <div v-if="displayEmail" class="card-subtitle">{{ displayEmail }}</div>
          <div v-else class="card-subtitle">{{ AUTH_TYPE_LABELS[account.auth_type] }}</div>
          <div class="card-meta-row">
            <span
              v-if="account.codex_plan_type"
              class="label-pill"
              :class="`label-pill-plan-${planTone}`"
            >
              {{ planLabel }}
            </span>
            <span v-if="account.is_default" class="label-pill label-pill-contrast">正在使用</span>
          </div>
        </div>
      </div>
      <div class="card-side">
        <n-dropdown
          trigger="click"
          :options="cardActionOptions"
          @select="handleCardActionSelect"
        >
          <n-button
            circle
            secondary
            size="small"
            class="card-icon-button"
            title="操作"
            aria-label="操作"
          >
            <template #icon>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                <path
                  d="M12 5h.01M12 12h.01M12 19h.01"
                  stroke="currentColor"
                  stroke-width="2.4"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
            </template>
          </n-button>
        </n-dropdown>
      </div>
    </div>

    <div v-if="hasCodexUsage(account)" class="usage-panel">
      <LazyAccountQuotaChart
        :five-hour="account.codex_usage_5h"
        :one-week="account.codex_usage_week"
        :plan-type="account.codex_plan_type"
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
import { computed, defineAsyncComponent, h } from 'vue'
import { AUTH_TYPE_LABELS } from '@/types'
import type { Account, CodexUsageWindow } from '@/types'
import StatusDot from '@/components/common/StatusDot.vue'
import type { DropdownOption } from 'naive-ui'
import {
  resolveAccountAvatarText,
  resolveAccountDisplayName,
} from '@/utils/account-display'
import {
  formatAccountPlanType,
  resolveAccountPlanTone,
} from '@/utils/account-plan'
import {
  resolveAccountStatusDiagnostic,
  resolveAccountStatusDisplay,
  resolveAccountStatusMessage,
} from '@/utils/account-status'

// 额度图表依赖 echarts，异步加载可以避免账号列表首屏把图表库打进同一路由 chunk。
const LazyAccountQuotaChart = defineAsyncComponent({
  loader: () => import('@/components/account/AccountQuotaChart.vue'),
  suspensible: false,
})

const props = defineProps<{
  account: Account
  checking?: boolean
  refreshingToken?: boolean
  triggeringConversation?: boolean
  warmupDisabled?: boolean
}>()

const emit = defineEmits<{
  detail: []
  check: []
  'refresh-token': []
  'switch-account': []
  'export-auth': []
  'trigger-warmup': []
  delete: []
}>()

type CardActionKey =
  | 'detail'
  | 'check'
  | 'refresh-token'
  | 'warmup-period'
  | 'switch-account'
  | 'export-auth'
  | 'delete'

const displayName = computed(() => resolveAccountDisplayName(props.account))
const displayAvatarText = computed(() => resolveAccountAvatarText(props.account))
const displayEmail = computed(() => {
  const email = props.account.email?.trim() ?? ''
  return email && email !== displayName.value ? email : ''
})
const displayUsageError = computed(() => formatUsageError(props.account.codex_usage_error))
const statusDisplay = computed(() => resolveAccountStatusDisplay(props.account))
const displayStatusMessage = computed(() => resolveAccountStatusMessage(props.account))
const statusDiagnostic = computed(() => resolveAccountStatusDiagnostic(props.account))
const canWarmupPeriod = computed(
  () =>
    isWarmupExecutableWindow(props.account.codex_usage_5h) ||
    isWarmupExecutableWindow(props.account.codex_usage_week),
)
const planLabel = computed(() => formatAccountPlanType(props.account.codex_plan_type))
const planTone = computed(() => resolveAccountPlanTone(props.account.codex_plan_type))
const cardActionOptions = computed<DropdownOption[]>(() => {
  const options: DropdownOption[] = [
    {
      label: '查看详情',
      key: 'detail',
      icon: () =>
        renderActionIcon(
          'M9 6h11M9 12h11M9 18h11M4 6h.01M4 12h.01M4 18h.01',
        ),
    },
    {
      label: props.checking ? '检测中' : '检测状态',
      key: 'check',
      disabled: props.checking,
      icon: () =>
        h(
          'svg',
          {
            width: 16,
            height: 16,
            viewBox: '0 0 24 24',
            fill: 'none',
          },
          [
            h('path', {
              d: 'M20 12a8 8 0 1 1-2.34-5.66',
              stroke: 'currentColor',
              'stroke-width': 1.8,
              'stroke-linecap': 'round',
              'stroke-linejoin': 'round',
            }),
            h('path', {
              d: 'M20 4v6h-6',
              stroke: 'currentColor',
              'stroke-width': 1.8,
              'stroke-linecap': 'round',
              'stroke-linejoin': 'round',
            }),
          ],
        ),
    },
    {
      label: props.refreshingToken ? '刷新中' : '刷新 Token',
      key: 'refresh-token',
      disabled: props.refreshingToken,
      icon: () => renderActionIcon('M21 12a9 9 0 1 1-2.64-6.36M21 3v6h-6'),
    },
    {
      label: '周期预热',
      key: 'warmup-period',
      disabled: props.triggeringConversation || props.warmupDisabled || !canWarmupPeriod.value,
      props: {
        title: '触发周期额度倒计时',
      },
      icon: () =>
        renderActionIcon(
          'M21 15a4 4 0 0 1-4 4H7l-4 4V7a4 4 0 0 1 4-4h10a4 4 0 0 1 4 4v8z',
        ),
    },
  ]

  options.push({
    label: '切换账号',
    key: 'switch-account',
    disabled: props.account.is_default,
    icon: () =>
      renderActionIcon(
        'M7 7h10l-3-3M17 17H7l3 3M17 7l-10 10',
      ),
  })

  options.push({
    label: '导出',
    key: 'export-auth',
    icon: () =>
      renderActionIcon(
        'M12 21V9M7 16l5 5 5-5M5 3h14',
      ),
  })

  options.push(
    {
      type: 'divider',
      key: 'card-action-divider',
    },
    {
      label: '删除账号',
      key: 'delete',
      icon: () =>
        renderActionIcon(
          'M4 7h16M9 11v6M15 11v6M10 4h4l1 2H9l1-2zm-3 3 1 12h8l1-12',
        ),
    },
  )

  return options
})

function hasCodexUsage(account: Account): boolean {
  return Boolean(account.codex_usage_5h || account.codex_usage_week)
}

function isWarmupExecutableWindow(window: CodexUsageWindow | undefined): boolean {
  if (!window) {
    return false
  }

  if (!Number.isFinite(window.reset_at) || !window.reset_at) {
    return false
  }

  const nowSeconds = Math.floor(Date.now() / 1000)
  const secondsUntilReset = window.reset_at - nowSeconds
  return secondsUntilReset >= window.window_seconds
}

function renderActionIcon(path: string) {
  return h(
    'svg',
    {
      width: 16,
      height: 16,
      viewBox: '0 0 24 24',
      fill: 'none',
    },
    h('path', {
      d: path,
      stroke: 'currentColor',
      'stroke-width': 1.8,
      'stroke-linecap': 'round',
      'stroke-linejoin': 'round',
    }),
  )
}

function handleCardActionSelect(key: string | number) {
  switch (key as CardActionKey) {
    case 'detail':
      emit('detail')
      break
    case 'check':
      emit('check')
      break
    case 'refresh-token':
      emit('refresh-token')
      break
    case 'warmup-period':
      emit('trigger-warmup')
      break
    case 'switch-account':
      emit('switch-account')
      break
    case 'export-auth':
      emit('export-auth')
      break
    case 'delete':
      emit('delete')
      break
  }
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
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  border-radius: 22px;
  border: 1px solid rgba(29, 29, 31, 0.08);
  background: var(--app-surface);
  box-shadow: var(--app-shadow);
  transition:
    border-color 0.2s ease,
    box-shadow 0.2s ease;
}

.account-card:hover {
  box-shadow: rgba(0, 0, 0, 0.22) 3px 9px 34px 0px;
}

.account-card.default {
  border-color: rgba(0, 113, 227, 0.46);
  background:
    linear-gradient(180deg, rgba(0, 113, 227, 0.08), rgba(0, 113, 227, 0.02) 42%),
    var(--app-feature-surface);
  color: var(--app-feature-ink);
  box-shadow:
    rgba(0, 113, 227, 0.2) 0 16px 42px 0,
    var(--app-shadow);
}

.card-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
}

.card-side {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.card-profile {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  min-width: 0;
}

.avatar {
  width: 40px;
  height: 40px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: #ffffff;
  font-family: var(--font-display);
  font-size: 15px;
  font-weight: 600;
}

.card-name-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 5px;
}

.card-copy {
  min-width: 0;
}

.card-name {
  margin: 0;
  min-width: 0;
  font-family: var(--font-display);
  font-size: 16px;
  line-height: 1.2;
  letter-spacing: 0.12px;
  font-weight: 700;
}

.card-subtitle {
  margin-top: 3px;
  font-size: 12px;
  line-height: 1.35;
  letter-spacing: -0.12px;
  color: var(--app-ink-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.card-meta-row {
  margin-top: 6px;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.account-card.default .card-subtitle {
  color: var(--app-feature-ink-secondary);
}

.label-pill {
  display: inline-flex;
  align-items: center;
  min-height: 22px;
  padding: 0 8px;
  border-radius: var(--app-radius-control);
  font-size: 10px;
  line-height: 1.33;
  letter-spacing: -0.12px;
}

.label-pill-contrast {
  background: rgba(29, 29, 31, 0.08);
  color: var(--app-ink);
}

.account-card.default .label-pill-contrast {
  background: var(--app-blue);
  color: #ffffff;
}

.label-pill-plan-green {
  background: rgba(52, 199, 89, 0.14);
  color: #248a3d;
}

.label-pill-plan-blue {
  background: rgba(0, 113, 227, 0.12);
  color: var(--app-blue);
}

.label-pill-plan-purple {
  background: rgba(139, 92, 246, 0.14);
  color: #6e44d9;
}

.label-pill-plan-neutral {
  background: rgba(29, 29, 31, 0.08);
  color: var(--app-ink);
}

.usage-panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.card-icon-button {
  flex-shrink: 0;
}

.status-diagnostic {
  margin-top: -6px;
  font-size: 11px;
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
