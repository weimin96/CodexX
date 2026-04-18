<template>
  <div class="app-shell">
    <!-- Custom Title Bar -->
    <div class="titlebar" data-tauri-drag-region>
      <div class="titlebar-left" data-tauri-drag-region>
        <div class="app-logo">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" stroke="#4f8ef7" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <span class="app-title">Codex Manager</span>
      </div>
      <div class="titlebar-right">
        <button class="win-btn" @click="minimizeWindow" title="最小化">
          <svg width="12" height="1" viewBox="0 0 12 1"><rect width="12" height="1" fill="currentColor"/></svg>
        </button>
        <button class="win-btn" @click="toggleMaximize" title="最大化">
          <svg width="10" height="10" viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9" rx="1" stroke="currentColor" fill="none"/></svg>
        </button>
        <button class="win-btn close" @click="closeWindow" title="关闭">
          <svg width="10" height="10" viewBox="0 0 10 10"><path d="M1 1l8 8M9 1l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
        </button>
      </div>
    </div>

    <!-- Main Content -->
    <div class="app-body">
      <!-- Sidebar -->
      <aside class="sidebar">
        <!-- Account selector chip -->
        <div class="account-chip" @click="router.push('/accounts')">
          <div
            class="account-avatar"
            :style="{ background: activeAccount?.color ?? '#4f8ef7' }"
          >
            {{ activeAccount?.avatar_text ?? '?' }}
          </div>
          <div class="account-chip-info">
            <div class="account-chip-name">{{ activeAccount?.name ?? '未选账号' }}</div>
            <div class="account-chip-type">{{ activeAccount ? AUTH_TYPE_LABELS[activeAccount.auth_type] : '—' }}</div>
          </div>
          <StatusDot :status="activeAccount?.status ?? 'unknown'" />
        </div>

        <n-divider style="margin: 8px 0;" />

        <!-- Navigation -->
        <n-menu
          :value="currentRoute"
          :options="menuOptions"
          :collapsed="false"
          :indent="16"
          @update:value="handleNav"
        />

        <div class="sidebar-footer">
          <div class="stat-pill">
            <span class="stat-label">账号总数</span>
            <n-badge :value="totalAccounts" :max="99" type="info" />
          </div>
          <div class="stat-pill">
            <span class="stat-label">异常</span>
            <n-badge
              :value="accountsByStatus['error']?.length ?? 0"
              :max="99"
              type="error"
            />
          </div>
        </div>
      </aside>

      <!-- Page View -->
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
import { computed, onMounted, h, Component } from 'vue'
import { useRouter, useRoute } from 'vue-router'
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

// Icons as SVG render functions
const renderIcon = (svgPath: string) => () =>
  h(NIcon, null, {
    default: () =>
      h('svg', { width: 18, height: 18, viewBox: '0 0 24 24', fill: 'none' }, [
        h('path', { d: svgPath, stroke: 'currentColor', 'stroke-width': 2, 'stroke-linecap': 'round', 'stroke-linejoin': 'round' }),
      ]),
  })

const menuOptions: MenuOption[] = [
  {
    label: '账号列表',
    key: 'AccountList',
    icon: renderIcon('M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8zm11 2v6m-3-3h6'),
  },
  {
    label: '用量统计',
    key: 'Usage',
    icon: renderIcon('M3 3v18h18M7 16l4-4 4 4 4-4'),
  },
  {
    label: '设置',
    key: 'Settings',
    icon: renderIcon('M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zm0 0v6m0-18v3M4.22 10.22l2.12 2.12M17.66 17.66l2.12 2.12M2 12h3m14 0h3M4.22 13.78l2.12-2.12M17.66 6.34l2.12-2.12'),
  },
]

function handleNav(key: string) {
  router.push({ name: key })
}

// Window controls
const appWindow = getCurrentWindow()
const minimizeWindow = () => appWindow.minimize()
const toggleMaximize = () => appWindow.toggleMaximize()
const closeWindow = () => appWindow.hide() // hide to tray instead of quit

// Listen for backend status events
onMounted(async () => {
  await accountStore.loadAccounts()

  await listen<{ account_id: string; status: string; message?: string }>(
    'account-status-updated',
    ({ payload }) => {
      accountStore.updateAccountStatusFromEvent(payload.account_id, payload.status, payload.message)
    },
  )
})
</script>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }

:root {
  --bg-primary: #0d1117;
  --bg-secondary: #161b22;
  --bg-tertiary: #1c2333;
  --border: #30363d;
  --text-primary: #e6edf3;
  --text-secondary: #8b949e;
  --accent: #4f8ef7;
  --accent-soft: rgba(79,142,247,0.12);
  --radius: 8px;
  --sidebar-w: 220px;
  --titlebar-h: 36px;
}

body {
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: Lato, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
}

.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg-primary);
  overflow: hidden;
  border-radius: 10px;
  border: 1px solid var(--border);
}

/* ── Titlebar ── */
.titlebar {
  height: var(--titlebar-h);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px 0 12px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  user-select: none;
  flex-shrink: 0;
  border-radius: 9px 9px 0 0;
}

.titlebar-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.app-logo { display: flex; align-items: center; }

.app-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  letter-spacing: 0.3px;
}

.titlebar-right {
  display: flex;
  align-items: center;
  gap: 2px;
}

.win-btn {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s, color 0.15s;
}
.win-btn:hover { background: rgba(255,255,255,0.08); color: var(--text-primary); }
.win-btn.close:hover { background: #da3633; color: #fff; }

/* ── Body ── */
.app-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* ── Sidebar ── */
.sidebar {
  width: var(--sidebar-w);
  flex-shrink: 0;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  padding: 12px 8px;
  gap: 4px;
  overflow-y: auto;
}

.account-chip {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 8px;
  border-radius: var(--radius);
  cursor: pointer;
  transition: background 0.15s;
}
.account-chip:hover { background: var(--accent-soft); }

.account-avatar {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  font-weight: 700;
  color: #fff;
  flex-shrink: 0;
}

.account-chip-info {
  flex: 1;
  min-width: 0;
}
.account-chip-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.account-chip-type {
  font-size: 11px;
  color: var(--text-secondary);
}

.sidebar-footer {
  margin-top: auto;
  padding-top: 12px;
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.stat-pill {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
}
.stat-label {
  font-size: 12px;
  color: var(--text-secondary);
}

/* ── Page ── */
.page-content {
  flex: 1;
  overflow-y: auto;
  background: var(--bg-primary);
}

/* ── Transitions ── */
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.18s ease;
}
.fade-slide-enter-from {
  opacity: 0;
  transform: translateY(6px);
}
.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

/* Scrollbar */
::-webkit-scrollbar { width: 5px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }
::-webkit-scrollbar-thumb:hover { background: #484f58; }
</style>
