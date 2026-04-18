<template>
  <n-config-provider
    :theme="naiveTheme"
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
import { computed, onMounted, watch } from 'vue'
import { darkTheme, lightTheme, zhCN, dateZhCN } from 'naive-ui'
import type { GlobalThemeOverrides } from 'naive-ui'
import AppLayout from '@/components/common/AppLayout.vue'
import { useSettingsStore } from '@/stores/settings'

const settingsStore = useSettingsStore()

const lightThemeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#0071e3',
    primaryColorHover: '#0077ed',
    primaryColorPressed: '#0066cc',
    primaryColorSuppl: '#0071e3',
    infoColor: '#0071e3',
    infoColorHover: '#0077ed',
    infoColorPressed: '#0066cc',
    successColor: '#1f8f5f',
    warningColor: '#b26a00',
    errorColor: '#c4314b',
    bodyColor: '#f5f5f7',
    cardColor: '#ffffff',
    modalColor: '#ffffff',
    popoverColor: '#ffffff',
    tableColor: '#ffffff',
    tableColorHover: '#fafafc',
    tableHeaderColor: '#fafafc',
    borderColor: 'rgba(29, 29, 31, 0.08)',
    dividerColor: 'rgba(29, 29, 31, 0.08)',
    inputColor: '#fafafc',
    actionColor: '#f5f5f7',
    textColorBase: '#1d1d1f',
    textColor1: '#1d1d1f',
    textColor2: 'rgba(29, 29, 31, 0.72)',
    textColor3: 'rgba(29, 29, 31, 0.56)',
    fontFamily:
      '"SF Pro Text", "SF Pro Display", -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", "Helvetica Neue", Arial, sans-serif',
    fontFamilyMono:
      '"SF Pro Text", "SF Pro Display", -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", "Helvetica Neue", Arial, sans-serif',
    borderRadius: '18px',
    borderRadiusSmall: '14px',
  },
  Button: {
    borderRadiusTiny: '980px',
    borderRadiusSmall: '980px',
    borderRadiusMedium: '980px',
    borderRadiusLarge: '980px',
    textColorPrimary: '#ffffff',
    textColorTextPrimary: '#1d1d1f',
    textColorTextHoverPrimary: '#1d1d1f',
    colorSecondary: '#ffffff',
    colorSecondaryHover: '#fafafc',
    colorSecondaryPressed: '#f1f1f3',
    borderSecondary: '1px solid rgba(29, 29, 31, 0.08)',
    heightSmall: '36px',
    heightMedium: '40px',
    heightLarge: '44px',
    paddingSmall: '0 16px',
    paddingMedium: '0 18px',
  },
  Card: {
    borderRadius: '28px',
    color: '#ffffff',
    borderColor: 'transparent',
    actionColor: '#fafafc',
  },
  Menu: {
    color: 'transparent',
    itemColorHover: 'rgba(255, 255, 255, 0.06)',
    itemColorActive: 'rgba(255, 255, 255, 0.08)',
    itemColorActiveHover: 'rgba(255, 255, 255, 0.12)',
    itemTextColor: 'rgba(255, 255, 255, 0.7)',
    itemTextColorHover: '#ffffff',
    itemTextColorActive: '#ffffff',
    itemTextColorActiveHover: '#ffffff',
    itemIconColor: 'rgba(255, 255, 255, 0.7)',
    itemIconColorHover: '#ffffff',
    itemIconColorActive: '#ffffff',
    itemIconColorActiveHover: '#ffffff',
    itemBorderRadius: '20px',
    borderRadius: '20px',
  },
  Tag: {
    borderRadius: '980px',
  },
  Input: {
    borderRadius: '18px',
    borderHover: '1px solid rgba(0, 113, 227, 0.28)',
    borderFocus: '1px solid #0071e3',
    boxShadowFocus: '0 0 0 2px rgba(0, 113, 227, 0.18)',
    color: '#fafafc',
    colorFocus: '#ffffff',
  },
  Select: {
    peers: {
      InternalSelection: {
        borderRadius: '18px',
        color: '#fafafc',
      },
    },
  },
  Modal: {
    borderRadius: '28px',
  },
  Radio: {
    buttonBorderRadius: '980px',
    buttonCheckedColor: '#1d1d1f',
    buttonCheckedTextColor: '#ffffff',
    buttonColor: '#fafafc',
    buttonTextColor: 'rgba(29, 29, 31, 0.72)',
  },
  Switch: {
    railColorActive: '#0071e3',
  },
  DataTable: {
    thColor: '#fafafc',
    tdColor: '#ffffff',
    tdColorHover: '#fafafc',
  },
}

const darkThemeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#2997ff',
    primaryColorHover: '#52abff',
    primaryColorPressed: '#0a84ff',
    primaryColorSuppl: '#2997ff',
    infoColor: '#2997ff',
    infoColorHover: '#52abff',
    infoColorPressed: '#0a84ff',
    successColor: '#32d583',
    warningColor: '#f5a524',
    errorColor: '#f97066',
    bodyColor: '#0f1012',
    cardColor: '#16181c',
    modalColor: '#16181c',
    popoverColor: '#16181c',
    tableColor: '#16181c',
    tableColorHover: '#1d2025',
    tableHeaderColor: '#1d2025',
    borderColor: 'rgba(255, 255, 255, 0.08)',
    dividerColor: 'rgba(255, 255, 255, 0.08)',
    inputColor: '#1d2025',
    actionColor: '#121417',
    textColorBase: '#f5f5f7',
    textColor1: '#f5f5f7',
    textColor2: 'rgba(255, 255, 255, 0.72)',
    textColor3: 'rgba(255, 255, 255, 0.56)',
    fontFamily:
      '"SF Pro Text", "SF Pro Display", -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", "Helvetica Neue", Arial, sans-serif',
    fontFamilyMono:
      '"SF Pro Text", "SF Pro Display", -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", "Helvetica Neue", Arial, sans-serif',
    borderRadius: '18px',
    borderRadiusSmall: '14px',
  },
  Button: {
    borderRadiusTiny: '980px',
    borderRadiusSmall: '980px',
    borderRadiusMedium: '980px',
    borderRadiusLarge: '980px',
    textColorPrimary: '#ffffff',
    textColorTextPrimary: '#f5f5f7',
    textColorTextHoverPrimary: '#f5f5f7',
    colorSecondary: '#1d2025',
    colorSecondaryHover: '#252932',
    colorSecondaryPressed: '#121417',
    borderSecondary: '1px solid rgba(255, 255, 255, 0.08)',
    heightSmall: '36px',
    heightMedium: '40px',
    heightLarge: '44px',
    paddingSmall: '0 16px',
    paddingMedium: '0 18px',
  },
  Card: {
    borderRadius: '28px',
    color: '#16181c',
    borderColor: 'transparent',
    actionColor: '#1d2025',
  },
  Menu: {
    color: 'transparent',
    itemColorHover: 'rgba(255, 255, 255, 0.06)',
    itemColorActive: 'rgba(255, 255, 255, 0.08)',
    itemColorActiveHover: 'rgba(255, 255, 255, 0.12)',
    itemTextColor: 'rgba(255, 255, 255, 0.7)',
    itemTextColorHover: '#ffffff',
    itemTextColorActive: '#ffffff',
    itemTextColorActiveHover: '#ffffff',
    itemIconColor: 'rgba(255, 255, 255, 0.7)',
    itemIconColorHover: '#ffffff',
    itemIconColorActive: '#ffffff',
    itemIconColorActiveHover: '#ffffff',
    itemBorderRadius: '20px',
    borderRadius: '20px',
  },
  Tag: {
    borderRadius: '980px',
  },
  Input: {
    borderRadius: '18px',
    borderHover: '1px solid rgba(41, 151, 255, 0.42)',
    borderFocus: '1px solid #2997ff',
    boxShadowFocus: '0 0 0 2px rgba(41, 151, 255, 0.2)',
    color: '#1d2025',
    colorFocus: '#22262d',
  },
  Select: {
    peers: {
      InternalSelection: {
        borderRadius: '18px',
        color: '#1d2025',
      },
    },
  },
  Modal: {
    borderRadius: '28px',
  },
  Radio: {
    buttonBorderRadius: '980px',
    buttonCheckedColor: '#f5f5f7',
    buttonCheckedTextColor: '#0f1012',
    buttonColor: '#1d2025',
    buttonTextColor: 'rgba(255, 255, 255, 0.72)',
  },
  Switch: {
    railColorActive: '#2997ff',
  },
  DataTable: {
    thColor: '#1d2025',
    tdColor: '#16181c',
    tdColorHover: '#1d2025',
  },
}

const currentTheme = computed(() => (settingsStore.settings.theme === 'dark' ? 'dark' : 'light'))
const naiveTheme = computed(() => (currentTheme.value === 'dark' ? darkTheme : lightTheme))
const themeOverrides = computed(() =>
  currentTheme.value === 'dark' ? darkThemeOverrides : lightThemeOverrides,
)

watch(
  currentTheme,
  (theme) => {
    if (typeof document === 'undefined') {
      return
    }

    document.documentElement.dataset.appTheme = theme
  },
  { immediate: true },
)

onMounted(async () => {
  try {
    await settingsStore.loadSettings()
  } catch (error) {
    console.warn('加载设置失败', error)
  }
})
</script>
