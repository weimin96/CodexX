import { defineStore } from 'pinia'
import { ref } from 'vue'
import { settingsService } from '@/services'
import type { AppSettings } from '@/types'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>({
    theme: 'dark',
    language: 'zh-CN',
    check_interval: '300',
    autostart: 'false',
    local_auth_auto_sync: 'true',
    local_auth_file_path: '',
  })
  const loading = ref(false)

  async function loadSettings() {
    loading.value = true
    try {
      settings.value = await settingsService.getSettings()
    } finally {
      loading.value = false
    }
  }

  async function saveSettings(updates: Partial<AppSettings>) {
    Object.assign(settings.value, updates)
    await settingsService.saveSettings(settings.value)
  }

  async function setAutostart(enabled: boolean) {
    await settingsService.setAutostart(enabled)
    settings.value.autostart = String(enabled)
  }

  return { settings, loading, loadSettings, saveSettings, setAutostart }
})
