import { defineStore } from 'pinia'
import { ref } from 'vue'
import { settingsService } from '@/services'
import type { AppSettings } from '@/types'

const SETTINGS_THEME_STORAGE_KEY = 'codex-manager.theme'

function readCachedTheme(): AppSettings['theme'] {
  if (typeof window === 'undefined') {
    return 'light'
  }

  const cachedTheme = window.localStorage.getItem(SETTINGS_THEME_STORAGE_KEY)
  return cachedTheme === 'dark' ? 'dark' : 'light'
}

function cacheTheme(theme: AppSettings['theme']) {
  if (typeof window === 'undefined') {
    return
  }

  window.localStorage.setItem(SETTINGS_THEME_STORAGE_KEY, theme)
}

function normalizeSettings(updates: Partial<AppSettings>): Partial<AppSettings> {
  if (!updates.theme) {
    return updates
  }

  return {
    ...updates,
    theme: updates.theme === 'dark' ? 'dark' : 'light',
  }
}

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>({
    theme: readCachedTheme(),
    language: 'zh-CN',
    check_interval: '300',
    autostart: 'false',
  })
  const loading = ref(false)
  const loaded = ref(false)
  let pendingLoad: Promise<void> | null = null

  async function loadSettings(force = false) {
    if (loaded.value && !force) {
      return
    }

    if (pendingLoad) {
      return pendingLoad
    }

    loading.value = true

    pendingLoad = (async () => {
      try {
        const nextSettings = normalizeSettings(await settingsService.getSettings())
        settings.value = {
          ...settings.value,
          ...nextSettings,
        }
        cacheTheme(settings.value.theme)
        loaded.value = true
      } finally {
        loading.value = false
        pendingLoad = null
      }
    })()

    return pendingLoad
  }

  async function saveSettings(updates: Partial<AppSettings>) {
    const normalizedUpdates = normalizeSettings(updates)
    const previousSettings = { ...settings.value }
    settings.value = {
      ...settings.value,
      ...normalizedUpdates,
    }
    cacheTheme(settings.value.theme)

    try {
      await settingsService.saveSettings(normalizedUpdates)
      loaded.value = true
    } catch (error) {
      settings.value = previousSettings
      cacheTheme(settings.value.theme)
      throw error
    }
  }

  async function setAutostart(enabled: boolean) {
    const previousAutostart = settings.value.autostart
    settings.value.autostart = String(enabled)

    try {
      await settingsService.setAutostart(enabled)
      loaded.value = true
    } catch (error) {
      settings.value.autostart = previousAutostart
      throw error
    }
  }

  return { settings, loading, loaded, loadSettings, saveSettings, setAutostart }
})
