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
          <span class="app-title">Codex Manager</span>
          <span class="app-caption">账号与统计</span>
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

    <div class="app-body">
      <aside class="sidebar">
        <div class="sidebar-top">
          <div class="sidebar-section-label">当前账号</div>
          <button class="account-chip" type="button" @click="router.push('/accounts')">
            <div
              class="account-avatar"
              :style="{ background: activeAccount?.color ?? '#0071e3' }"
            >
              {{ activeAccount?.avatar_text ?? '?' }}
            </div>
            <div class="account-chip-info">
              <div class="account-chip-name">{{ activeAccount?.name ?? '未选账号' }}</div>
              <div class="account-chip-type">
                {{ activeAccount ? AUTH_TYPE_LABELS[activeAccount.auth_type] : '等待选择' }}
              </div>
            </div>
            <StatusDot :status="activeAccount?.status ?? 'unknown'" />
          </button>
        </div>

        <div class="sidebar-navigation">
          <div class="sidebar-section-label">导航</div>
          <n-menu
            :value="currentRoute"
            :options="menuOptions"
            :collapsed="false"
            :indent="20"
            @update:value="handleNav"
          />
        </div>

        <div class="sidebar-footer">
          <div class="sidebar-stat-card">
            <span class="sidebar-stat-label">账号总数</span>
            <strong class="sidebar-stat-value">{{ totalAccounts }}</strong>
          </div>
          <div class="sidebar-stat-card">
            <span class="sidebar-stat-label">异常账号</span>
            <strong class="sidebar-stat-value">
              {{ accountsByStatus.error?.length ?? 0 }}
            </strong>
          </div>
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
import { isTauri as detectTauriRuntime } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import type { MenuOption } from 'naive-ui'
import { NIcon } from 'naive-ui'
import { useAccountStore } from '@/stores/account'
import { AUTH_TYPE_LABELS } from '@/types'
import StatusDot from '@/components/common/StatusDot.vue'

const router = useRouter()
const route = useRoute()
const accountStore = useAccountStore()
const { activeAccount, totalAccounts, accountsByStatus } = storeToRefs(accountStore)

const currentRoute = computed(() => route.name as string)

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
    label: '账号列表',
    key: 'AccountList',
    icon: renderIcon(
      'M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8zm11 2v6m-3-3h6',
    ),
  },
  {
    label: '用量统计',
    key: 'Usage',
    icon: renderIcon('M3 3v18h18M7 16l4-4 4 4 4-4'),
  },
  {
    label: '设置',
    key: 'Settings',
    icon: renderIcon(
      'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zm0 0v6m0-18v3M4.22 10.22l2.12 2.12M17.66 17.66l2.12 2.12M2 12h3m14 0h3M4.22 13.78l2.12-2.12M17.66 6.34l2.12-2.12',
    ),
  },
]

const currentSectionLabel = computed(() => {
  const currentOption = menuOptions.find((option) => option.key === currentRoute.value)
  return typeof currentOption?.label === 'string' ? currentOption.label : 'Codex Manager'
})

function handleNav(key: string) {
  router.push({ name: key })
}

const nonDragSelector = 'button, a, input, textarea, select, [role="button"]'
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

  await accountStore.loadAccounts()

  if (isTauri.value) {
    try {
      await listen<{ account_id: string; status: string; message?: string }>(
        'account-status-updated',
        ({ payload }) => {
          accountStore.updateAccountStatusFromEvent(
            payload.account_id,
            payload.status,
            payload.message,
          )
        },
      )
    } catch (error) {
      console.warn('状态事件监听失败', error)
    }
  }
})
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
  background: rgba(0, 0, 0, 0.82);
  color: rgba(255, 255, 255, 0.78);
  backdrop-filter: saturate(180%) blur(20px);
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
  background: rgba(255, 255, 255, 0.08);
  color: #ffffff;
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
  color: #ffffff;
}

.app-caption {
  font-size: 10px;
  line-height: 1.47;
  letter-spacing: -0.08px;
  color: rgba(255, 255, 255, 0.56);
}

.titlebar-center {
  justify-self: center;
  font-size: 11px;
  line-height: 1.33;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: rgba(255, 255, 255, 0.64);
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
  color: rgba(255, 255, 255, 0.72);
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
  background: rgba(255, 255, 255, 0.08);
  color: #ffffff;
  transform: translateY(-1px);
}

.win-btn.close:hover {
  background: rgba(196, 49, 75, 0.88);
}

.app-body {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: var(--app-sidebar-width) minmax(0, 1fr);
  overflow: hidden;
}

.sidebar {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 18px 14px 14px;
  background: var(--app-black);
  color: rgba(255, 255, 255, 0.78);
  min-height: 0;
  overflow: hidden;
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
  color: rgba(255, 255, 255, 0.56);
}

.account-chip {
  width: 100%;
  border: none;
  border-radius: 20px;
  padding: 12px;
  display: flex;
  align-items: center;
  gap: 12px;
  background: var(--app-dark-surface-soft);
  color: inherit;
  text-align: left;
  cursor: pointer;
  transition:
    transform 0.18s ease,
    background-color 0.18s ease;
}

.account-chip:hover {
  transform: translateY(-1px);
  background: var(--app-dark-surface-elevated);
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
  color: #ffffff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.account-chip-type {
  margin-top: 2px;
  font-size: 11px;
  line-height: 1.33;
  color: rgba(255, 255, 255, 0.56);
}

.sidebar-navigation :deep(.n-menu-item-content) {
  min-height: 40px;
}

.sidebar-footer {
  margin-top: auto;
  display: grid;
  gap: 8px;
}

.sidebar-stat-card {
  padding: 12px;
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.06);
}

.sidebar-stat-label {
  display: block;
  font-size: 11px;
  line-height: 1.33;
  color: rgba(255, 255, 255, 0.56);
}

.sidebar-stat-value {
  display: block;
  margin-top: 4px;
  font-family: var(--font-display);
  font-size: 20px;
  line-height: 1.2;
  letter-spacing: 0.12px;
  color: #ffffff;
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

  .sidebar {
    gap: 12px;
    padding-bottom: 12px;
    overflow-y: auto;
  }

  .sidebar-footer {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 640px) {
  .titlebar {
    padding: 0 10px 0 12px;
  }

  .brand-copy {
    display: none;
  }

  .sidebar-footer {
    grid-template-columns: 1fr;
  }
}
</style>
