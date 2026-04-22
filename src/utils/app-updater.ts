import { computed, readonly, ref } from 'vue'
import { isTauri as detectTauriRuntime } from '@tauri-apps/api/core'
import { rememberInstalledUpdateChangelog } from '@/utils/update-changelog'

export interface AvailableAppUpdate {
  version: string
  body?: string
}

export type AppUpdateOutcome =
  | { status: 'unsupported' }
  | { status: 'not_available' }
  | { status: 'available'; version: string; body?: string }
  | { status: 'installed'; version: string; body?: string }

const availableAppUpdateState = ref<AvailableAppUpdate | null>(null)

export function useAvailableAppUpdate() {
  return {
    availableAppUpdate: readonly(availableAppUpdateState),
    hasAvailableAppUpdate: computed(() => availableAppUpdateState.value !== null),
  }
}

export function clearAvailableAppUpdate() {
  availableAppUpdateState.value = null
}

function rememberAvailableAppUpdate(update: AvailableAppUpdate) {
  availableAppUpdateState.value = { ...update }
}

export async function checkAppUpdate(options: { rememberAvailable?: boolean } = {}): Promise<AppUpdateOutcome> {
  if (!detectTauriRuntime()) {
    if (options.rememberAvailable) {
      clearAvailableAppUpdate()
    }
    return { status: 'unsupported' }
  }

  const { check } = await import('@tauri-apps/plugin-updater')
  const update = await check()
  if (!update) {
    if (options.rememberAvailable) {
      clearAvailableAppUpdate()
    }
    return { status: 'not_available' }
  }

  const availableUpdate: AvailableAppUpdate = {
    version: update.version,
    body: update.body,
  }

  if (options.rememberAvailable) {
    rememberAvailableAppUpdate(availableUpdate)
  }

  return {
    status: 'available',
    ...availableUpdate,
  }
}

export async function installAppUpdate(): Promise<AppUpdateOutcome> {
  if (!detectTauriRuntime()) {
    clearAvailableAppUpdate()
    return { status: 'unsupported' }
  }

  const [{ check }, { relaunch }] = await Promise.all([
    import('@tauri-apps/plugin-updater'),
    import('@tauri-apps/plugin-process'),
  ])
  const update = await check()
  if (!update) {
    clearAvailableAppUpdate()
    return { status: 'not_available' }
  }

  await update.downloadAndInstall()
  clearAvailableAppUpdate()
  rememberInstalledUpdateChangelog({
    version: update.version,
    body: update.body,
  })
  await relaunch()

  return {
    status: 'installed',
    version: update.version,
    body: update.body,
  }
}
