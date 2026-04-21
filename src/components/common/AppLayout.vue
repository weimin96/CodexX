<template>
  <div class="app-shell">
    <div
      class="window-resize-zone window-resize-zone-top"
      @mousedown.prevent="handleResizeZoneMouseDown('North', $event)"
    />
    <div
      class="window-resize-zone window-resize-zone-left"
      @mousedown.prevent="handleResizeZoneMouseDown('West', $event)"
    />
    <div
      class="window-resize-zone window-resize-zone-right"
      @mousedown.prevent="handleResizeZoneMouseDown('East', $event)"
    />
    <div
      class="window-resize-zone window-resize-zone-bottom"
      @mousedown.prevent="handleResizeZoneMouseDown('South', $event)"
    />
    <div
      class="window-resize-zone window-resize-zone-top-left"
      @mousedown.prevent="handleResizeZoneMouseDown('NorthWest', $event)"
    />
    <div
      class="window-resize-zone window-resize-zone-top-right"
      @mousedown.prevent="handleResizeZoneMouseDown('NorthEast', $event)"
    />
    <div
      class="window-resize-zone window-resize-zone-bottom-left"
      @mousedown.prevent="handleResizeZoneMouseDown('SouthWest', $event)"
    />
    <div
      class="window-resize-zone window-resize-zone-bottom-right"
      @mousedown.prevent="handleResizeZoneMouseDown('SouthEast', $event)"
    />

    <div class="titlebar" @mousedown="handleTitlebarMouseDown">
      <div class="titlebar-left">
        <div class="app-logo" aria-hidden="true">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
            <path
              d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </div>
        <div class="brand-copy">
          <span class="app-title">CodexX</span>
        </div>
      </div>

      <div class="titlebar-center">
        {{ currentSectionLabel }}
      </div>

      <div class="titlebar-right">
        <button class="win-btn" @click="minimizeWindow" title="最小化">
          <svg width="12" height="1" viewBox="0 0 12 1">
            <rect width="12" height="1" fill="currentColor" />
          </svg>
        </button>
        <button class="win-btn" @click="toggleMaximize" title="最大化">
          <svg width="10" height="10" viewBox="0 0 10 10">
            <rect
              x="0.5"
              y="0.5"
              width="9"
              height="9"
              rx="1"
              stroke="currentColor"
              fill="none"
            />
          </svg>
        </button>
        <button class="win-btn close" @click="closeWindow" title="关闭">
          <svg width="10" height="10" viewBox="0 0 10 10">
            <path
              d="M1 1l8 8M9 1l-8 8"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
        </button>
      </div>
    </div>

    <div class="app-body" :class="{ 'app-body-collapsed': sidebarCollapsed }">
      <aside class="sidebar" :class="{ collapsed: sidebarCollapsed }">
        <div class="sidebar-top">
          <div class="sidebar-section-header" :class="{ collapsed: sidebarCollapsed }">
            <div v-if="!sidebarCollapsed" class="sidebar-section-label">当前账号</div>
            <button
              class="sidebar-toggle-button"
              type="button"
              :title="sidebarCollapsed ? '展开侧边栏' : '收起侧边栏'"
              :aria-label="sidebarCollapsed ? '展开侧边栏' : '收起侧边栏'"
              @click="sidebarCollapsed = !sidebarCollapsed"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                <path
                  v-if="sidebarCollapsed"
                  d="M9 6l6 6-6 6"
                  stroke="currentColor"
                  stroke-width="1.8"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
                <path
                  v-else
                  d="M15 6l-6 6 6 6"
                  stroke="currentColor"
                  stroke-width="1.8"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
            </button>
          </div>

          <button
            class="account-chip"
            :class="{ collapsed: sidebarCollapsed }"
            type="button"
            :title="activeAccountDisplayName"
            @click="router.push('/accounts')"
          >
            <div
              class="account-avatar"
              :style="{ background: activeAccount?.color ?? '#0071e3' }"
            >
              {{ activeAccountAvatarText }}
            </div>
            <div v-if="!sidebarCollapsed" class="account-chip-info">
              <div class="account-chip-name">{{ activeAccountDisplayName }}</div>
              <div class="account-chip-type">
                {{ activeAccount ? AUTH_TYPE_LABELS[activeAccount.auth_type] : '等待选择' }}
              </div>
            </div>
            <StatusDot
              v-if="!sidebarCollapsed"
              :status="activeAccountStatusDisplay.tone"
              :label="activeAccountStatusDisplay.label"
              :title="activeAccountStatusDisplay.title"
            />
          </button>
        </div>

        <div class="sidebar-navigation">
          <div v-if="!sidebarCollapsed" class="sidebar-section-label">导航</div>
          <n-menu
            :value="currentRoute"
            :options="menuOptions"
            :collapsed="sidebarCollapsed"
            :collapsed-width="62"
            :collapsed-icon-size="18"
            :indent="20"
            @update:value="handleNav"
          />
        </div>

        <div class="sidebar-footer">
          <button
            class="codex-launch-button"
            :class="{ active: currentRoute === 'CodexLaunch', collapsed: sidebarCollapsed }"
            type="button"
            :title="sidebarCollapsed ? '启动器' : undefined"
            @click="router.push({ name: 'CodexLaunch' })"
          >
            <span class="codex-launch-icon">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                <path
                  d="M8 9l-4 3 4 3m8-6 4 3-4 3M14 5l-4 14"
                  stroke="currentColor"
                  stroke-width="1.8"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
            </span>
            <span v-if="!sidebarCollapsed">启动器</span>
          </button>
        </div>
      </aside>

      <main class="page-content">
        <router-view v-slot="{ Component }">
          <transition name="fade-slide" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, h, onMounted, ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { storeToRefs } from 'pinia'
import { isTauri as detectTauriRuntime } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import type { MenuOption } from 'naive-ui'
import { NIcon, useDialog } from 'naive-ui'
import { useAccountStore } from '@/stores/account'
import { useSettingsStore } from '@/stores/settings'
import { AUTH_TYPE_LABELS } from '@/types'
import type { CodexQuotaExhaustedEvent } from '@/types'
import StatusDot from '@/components/common/StatusDot.vue'
import { resolveAccountAvatarText, resolveAccountDisplayName } from '@/utils/account-display'
import { resolveAccountStatusDisplay } from '@/utils/account-status'
import { checkAppUpdate, installAppUpdate } from '@/utils/app-updater'

const router = useRouter()
const route = useRoute()
const dialog = useDialog()
const accountStore = useAccountStore()
const settingsStore = useSettingsStore()
const { activeAccount } = storeToRefs(accountStore)
const quotaAlertShownKeys = new Set<string>()
let startupAccountRefreshStarted = false

const currentRoute = computed(() => route.name as string)
const activeAccountDisplayName = computed(() =>
  activeAccount.value ? resolveAccountDisplayName(activeAccount.value) : '未选账号',
)
const activeAccountAvatarText = computed(() =>
  activeAccount.value ? resolveAccountAvatarText(activeAccount.value) : '?',
)
const activeAccountStatusDisplay = computed(() => resolveAccountStatusDisplay(activeAccount.value))

const renderIcon = (svgPath: string) => () =>
  h(NIcon, null, {
    default: () =>
      h('svg', { width: 18, height: 18, viewBox: '0 0 24 24', fill: 'none' }, [
        h('path', {
          d: svgPath,
          stroke: 'currentColor',
          'stroke-width': 1.8,
          'stroke-linecap': 'round',
          'stroke-linejoin': 'round',
        }),
      ]),
  })

const menuOptions: MenuOption[] = [
  {
    label: '仪表盘',
    key: 'Usage',
    icon: renderIcon(
      'M4 13a8 8 0 1 1 16 0M12 13l4-4M12 13v5',
    ),
  },
  {
    label: '账号列表',
    key: 'AccountList',
    icon: renderIcon(
      'M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8zm11 2v6m-3-3h6',
    ),
  },
  {
    label: 'Codex 配置',
    key: 'CodexConfig',
    icon: renderIcon('M4 5h16M4 12h16M4 19h10M8 3v4M16 10v4M12 17v4'),
  },
  {
    label: '设置',
    key: 'Settings',
    icon: renderIcon(
      'M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7zm8 3.5-.92-.53a7.9 7.9 0 0 0-.42-1.03l.53-.92a1 1 0 0 0-.16-1.23l-1.01-1.01a1 1 0 0 0-1.23-.16l-.92.53c-.33-.17-.67-.31-1.03-.42L14 4.33a1 1 0 0 0-.98-.83h-1.04a1 1 0 0 0-.98.83l-.2 1.06c-.36.11-.7.25-1.03.42l-.92-.53a1 1 0 0 0-1.23.16L6.61 6.45a1 1 0 0 0-.16 1.23l.53.92c-.17.33-.31.67-.42 1.03L5.5 10a1 1 0 0 0-.83.98v1.04a1 1 0 0 0 .83.98l1.06.2c.11.36.25.7.42 1.03l-.53.92a1 1 0 0 0 .16 1.23l1.01 1.01a1 1 0 0 0 1.23.16l.92-.53c.33.17.67.31 1.03.42l.2 1.06a1 1 0 0 0 .98.83h1.04a1 1 0 0 0 .98-.83l.2-1.06c.36-.11.7-.25 1.03-.42l.92.53a1 1 0 0 0 1.23-.16l1.01-1.01a1 1 0 0 0 .16-1.23l-.53-.92c.17-.33.31-.67.42-1.03l1.06-.2a1 1 0 0 0 .83-.98v-1.04A1 1 0 0 0 20 12z',
    ),
  },
]

const currentSectionLabel = computed(() => {
  if (currentRoute.value === 'CodexLaunch') {
    return '启动器'
  }

  const currentOption = menuOptions.find((option) => option.key === currentRoute.value)
  return typeof currentOption?.label === 'string' ? currentOption.label : 'CodexX'
})

function handleNav(key: string) {
  router.push({ name: key })
}

const nonDragSelector = 'button, a, input, textarea, select, [role="button"]'
const QUOTA_EXHAUSTED_THRESHOLD = 99.9
type WindowResizeDirection =
  | 'North'
  | 'South'
  | 'East'
  | 'West'
  | 'NorthEast'
  | 'NorthWest'
  | 'SouthEast'
  | 'SouthWest'

let appWindow: ReturnType<typeof getCurrentWindow> | null = null
const isTauri = ref(detectTauriRuntime())
const sidebarCollapsed = ref(false)

const minimizeWindow = () => appWindow?.minimize()
const toggleMaximize = () => appWindow?.toggleMaximize()
const closeWindow = () => appWindow?.hide()

function handleTitlebarMouseDown(event: MouseEvent) {
  if (!isTauri.value || event.button !== 0 || !appWindow) return

  const target = event.target as HTMLElement | null
  if (target?.closest(nonDragSelector)) return

  void appWindow.startDragging().catch((error) => {
    console.warn('窗口拖动启动失败', error)
  })
}

function handleResizeZoneMouseDown(direction: WindowResizeDirection, event: MouseEvent) {
  if (!isTauri.value || event.button !== 0 || !appWindow) return

  void appWindow.startResizeDragging(direction).catch((error) => {
    console.warn(`窗口缩放启动失败: ${direction}`, error)
  })
}

onMounted(async () => {
  if (isTauri.value) {
    appWindow = getCurrentWindow()
  } else {
    console.warn('当前不在 Tauri 环境中，窗口控制已禁用')
  }

  await Promise.all([accountStore.loadAccounts(), settingsStore.loadSettings()])
  void runStartupAutoUpdateCheck()

  if (isTauri.value) {
    try {
      await listen<{ account_id: string; status: string; message?: string; account?: import('@/types').Account }>(
        'account-status-updated',
        ({ payload }) => {
          accountStore.updateAccountStatusFromEvent(
            payload.account_id,
            payload.status,
            payload.message,
            payload.account,
          )
        },
      )
      await listen<{ account_id: string }>('default-account-updated', () => {
        void accountStore.loadAccounts()
      })
      await listen<CodexQuotaExhaustedEvent>('codex-quota-exhausted', ({ payload }) => {
        handleQuotaExhausted(payload)
      })
    } catch (error) {
      console.warn('状态事件监听失败', error)
    }
  }

  void refreshAccountsOnFirstStartup()
})

function handleQuotaExhausted(payload: CodexQuotaExhaustedEvent) {
  if (settingsStore.settings.quota_alert_enabled !== 'true') {
    return
  }

  const exhaustedWindows = [
    payload.five_hour_used_percent !== undefined &&
    payload.five_hour_used_percent >= QUOTA_EXHAUSTED_THRESHOLD
      ? '5 小时额度'
      : '',
    payload.weekly_used_percent !== undefined &&
    payload.weekly_used_percent >= QUOTA_EXHAUSTED_THRESHOLD
      ? '7 天额度'
      : '',
  ].filter(Boolean)

  if (exhaustedWindows.length === 0) {
    return
  }

  const alertKey = [
    payload.account_id,
    payload.task_label,
    exhaustedWindows.join(','),
  ].join('|')
  if (quotaAlertShownKeys.has(alertKey)) {
    return
  }
  quotaAlertShownKeys.add(alertKey)

  const planText = payload.plan_type ? `，计划 ${payload.plan_type.toUpperCase()}` : ''
  dialog.warning({
    title: 'Codex 额度已用尽',
    content: `${payload.task_label}完成后检测到账号「${payload.account_name}」${exhaustedWindows.join('、')}已用尽${planText}。请切换到仍有剩余额度的账号后继续任务。`,
    positiveText: '去切换账号',
    negativeText: '稍后处理',
    onPositiveClick: () => {
      void router.push({ name: 'AccountList' })
    },
  })
}

async function runStartupAutoUpdateCheck() {
  if (!isTauri.value || settingsStore.settings.auto_update_enabled !== 'true') {
    return
  }

  try {
    const outcome = await checkAppUpdate()
    if (outcome.status !== 'available') {
      return
    }

    dialog.info({
      title: `发现新版本 ${outcome.version}`,
      content: outcome.body?.trim() || '可以在线下载并安装，安装完成后应用会重启。',
      positiveText: '下载并重启',
      negativeText: '稍后处理',
      onPositiveClick: async () => {
        try {
          await installAppUpdate()
        } catch (error) {
          console.warn('自动更新安装失败', error)
          dialog.error({
            title: '自动更新失败',
            content: '更新下载或安装失败，请稍后在设置页手动检查。',
            positiveText: '知道了',
          })
        }
      },
    })
  } catch (error) {
    console.warn('启动自动检查更新失败', error)
  }
}

async function refreshAccountsOnFirstStartup() {
  if (startupAccountRefreshStarted || accountStore.accounts.length === 0) {
    return
  }

  startupAccountRefreshStarted = true
  try {
    await accountStore.checkAllStatus()
  } catch (error) {
    console.warn('首次启动刷新账号信息失败', error)
  }
}
</script>

<style scoped>
.app-shell {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100dvh;
  max-height: 100dvh;
  background: var(--app-canvas);
  overflow: hidden;
}

.window-resize-zone {
  position: absolute;
  z-index: 30;
  pointer-events: auto;
}

.window-resize-zone-top,
.window-resize-zone-bottom {
  left: 14px;
  right: 14px;
  height: 6px;
}

.window-resize-zone-left,
.window-resize-zone-right {
  top: 14px;
  bottom: 14px;
  width: 6px;
}

.window-resize-zone-top {
  top: 0;
  cursor: ns-resize;
}

.window-resize-zone-bottom {
  bottom: 0;
  cursor: ns-resize;
}

.window-resize-zone-left {
  left: 0;
  cursor: ew-resize;
}

.window-resize-zone-right {
  right: 0;
  cursor: ew-resize;
}

.window-resize-zone-top-left,
.window-resize-zone-top-right,
.window-resize-zone-bottom-left,
.window-resize-zone-bottom-right {
  width: 14px;
  height: 14px;
}

.window-resize-zone-top-left {
  top: 0;
  left: 0;
  cursor: nwse-resize;
}

.window-resize-zone-top-right {
  top: 0;
  right: 0;
  cursor: nesw-resize;
}

.window-resize-zone-bottom-left {
  bottom: 0;
  left: 0;
  cursor: nesw-resize;
}

.window-resize-zone-bottom-right {
  right: 0;
  bottom: 0;
  cursor: nwse-resize;
}

.titlebar {
  height: var(--app-titlebar-height);
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: center;
  gap: 12px;
  padding: 0 12px 0 16px;
  background: var(--app-titlebar-bg);
  color: var(--app-titlebar-ink);
  backdrop-filter: saturate(180%) blur(20px);
  border-bottom: 1px solid var(--app-sidebar-border);
  flex-shrink: 0;
}

.titlebar-left,
.titlebar-right {
  display: flex;
  align-items: center;
}

.titlebar-left {
  gap: 10px;
}

.app-logo {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--app-titlebar-hover);
  color: var(--app-titlebar-ink-strong);
}

.brand-copy {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.app-title {
  font-size: 12px;
  line-height: 1.33;
  letter-spacing: -0.12px;
  font-weight: 600;
  color: var(--app-titlebar-ink-strong);
}

.titlebar-center {
  justify-self: center;
  font-size: 11px;
  line-height: 1.33;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--app-titlebar-muted);
}

.titlebar-right {
  justify-content: flex-end;
  gap: 4px;
}

.win-btn {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: var(--app-titlebar-ink);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition:
    background-color 0.18s ease,
    color 0.18s ease,
    transform 0.18s ease;
}

.win-btn:hover {
  background: var(--app-titlebar-hover);
  color: var(--app-titlebar-ink-strong);
  transform: translateY(-1px);
}

.win-btn.close:hover {
  background: var(--app-titlebar-close-hover);
}

.app-body {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: var(--app-sidebar-width) minmax(0, 1fr);
  overflow: hidden;
}

.app-body.app-body-collapsed {
  grid-template-columns: 86px minmax(0, 1fr);
}

.sidebar {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 18px 14px 14px;
  background: var(--app-sidebar-bg);
  color: var(--app-sidebar-ink);
  border-right: 1px solid var(--app-sidebar-border);
  min-height: 0;
  overflow: hidden;
}

.sidebar.collapsed {
  padding-inline: 12px;
}

.sidebar-section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.sidebar-section-header.collapsed {
  justify-content: center;
}

.sidebar-toggle-button {
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: var(--app-sidebar-panel);
  color: var(--app-sidebar-ink-strong);
  cursor: pointer;
  transition:
    background-color 0.18s ease,
    color 0.18s ease,
    transform 0.18s ease;
}

.sidebar-toggle-button:hover {
  background: var(--app-sidebar-panel-hover);
  color: var(--app-blue);
  transform: translateY(-1px);
}

.sidebar-top,
.sidebar-navigation {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sidebar-section-label {
  font-size: 10px;
  line-height: 1.33;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: var(--app-sidebar-muted);
}

.account-chip {
  width: 100%;
  border: none;
  border-radius: 20px;
  padding: 12px;
  display: flex;
  align-items: center;
  gap: 12px;
  background: var(--app-sidebar-panel);
  color: inherit;
  text-align: left;
  cursor: pointer;
  transition:
    transform 0.18s ease,
    background-color 0.18s ease;
}

.account-chip:hover {
  transform: translateY(-1px);
  background: var(--app-sidebar-panel-hover);
}

.account-chip.collapsed {
  justify-content: center;
  padding: 12px 10px;
}

.account-avatar {
  width: 36px;
  height: 36px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  font-weight: 700;
  color: #ffffff;
  flex-shrink: 0;
}

.account-chip-info {
  flex: 1;
  min-width: 0;
}

.account-chip-name {
  font-size: 14px;
  line-height: 1.24;
  font-weight: 600;
  color: var(--app-sidebar-ink-strong);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.account-chip-type {
  margin-top: 2px;
  font-size: 11px;
  line-height: 1.33;
  color: var(--app-sidebar-muted);
}

.sidebar-navigation :deep(.n-menu-item-content) {
  min-height: 40px;
}

.sidebar.collapsed .sidebar-navigation :deep(.n-menu-item-content) {
  justify-content: center;
  padding-inline: 0 !important;
  padding-right: 0 !important;
  grid-template-areas: "icon";
  grid-template-columns: 1fr;
  justify-items: center;
}

.sidebar.collapsed .sidebar-navigation :deep(.n-menu-item-content__icon) {
  margin-right: 0 !important;
}

.sidebar.collapsed .sidebar-navigation :deep(.n-menu-item-content-header),
.sidebar.collapsed .sidebar-navigation :deep(.n-menu-item-content__arrow) {
  display: none !important;
}

.sidebar-footer {
  margin-top: auto;
}

.codex-launch-button {
  width: 100%;
  min-height: 42px;
  border: none;
  border-radius: 16px;
  padding: 0 12px;
  display: flex;
  align-items: center;
  gap: 10px;
  background: var(--app-sidebar-panel);
  color: var(--app-sidebar-ink-strong);
  font-size: 13px;
  line-height: 1.33;
  font-weight: 600;
  cursor: pointer;
  transition:
    background-color 0.18s ease,
    color 0.18s ease,
    transform 0.18s ease;
}

.codex-launch-button.collapsed {
  justify-content: center;
  padding-inline: 0;
}

.codex-launch-button:hover,
.codex-launch-button.active {
  background: var(--app-sidebar-panel-hover);
  color: var(--app-blue);
  transform: translateY(-1px);
}

.codex-launch-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.page-content {
  display: flex;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  background: var(--app-canvas);
}

.page-content :deep(.app-page) {
  flex: 1;
  min-height: 0;
}

.fade-slide-enter-active,
.fade-slide-leave-active {
  transition:
    opacity 0.24s ease,
    transform 0.24s ease;
}

.fade-slide-enter-from,
.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(12px);
}

@media (max-width: 960px) {
  .titlebar {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .titlebar-center {
    display: none;
  }

  .app-body {
    grid-template-columns: 1fr;
  }

  .app-body.app-body-collapsed {
    grid-template-columns: 1fr;
  }

  .sidebar {
    gap: 12px;
    padding-bottom: 12px;
    overflow-y: auto;
  }

}

@media (max-width: 640px) {
  .titlebar {
    padding: 0 10px 0 12px;
  }

  .brand-copy {
    display: none;
  }

}
</style>
