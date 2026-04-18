<template>
  <n-config-provider
    :theme="darkTheme"
    :theme-overrides="themeOverrides"
    :locale="zhCN"
    :date-locale="dateZhCN"
  >
    <n-message-provider>
      <n-notification-provider>
        <n-dialog-provider>
          <AppLayout />
        </n-dialog-provider>
      </n-notification-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { darkTheme, zhCN, dateZhCN } from 'naive-ui'
import type { GlobalThemeOverrides } from 'naive-ui'
import AppLayout from '@/components/common/AppLayout.vue'

const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#4f8ef7',
    primaryColorHover: '#6ba3f9',
    primaryColorPressed: '#3a75e8',
    primaryColorSuppl: '#4f8ef7',
    bodyColor: '#0d1117',
    cardColor: '#161b22',
    modalColor: '#161b22',
    popoverColor: '#1c2333',
    tableColor: '#161b22',
    tableColorHover: '#1c2333',
    borderColor: '#30363d',
    dividerColor: '#21262d',
    inputColor: '#0d1117',
    fontFamily: 'Lato, -apple-system, BlinkMacSystemFont, sans-serif',
    fontFamilyMono: '"Fira Code", monospace',
    borderRadius: '8px',
    borderRadiusSmall: '6px',
  },
  Button: {
    borderRadiusMedium: '8px',
  },
  Card: {
    borderRadius: '12px',
    color: '#161b22',
  },
  Menu: {
    color: 'transparent',
    itemColorActive: 'rgba(79,142,247,0.12)',
    itemColorActiveHover: 'rgba(79,142,247,0.18)',
    itemTextColorActive: '#4f8ef7',
    itemIconColorActive: '#4f8ef7',
    borderRadius: '8px',
  },
  Tag: {
    borderRadius: '6px',
  },
}
</script>
