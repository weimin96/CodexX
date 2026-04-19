import { isTauri as detectTauriRuntime } from '@tauri-apps/api/core'

export type AppUpdateOutcome =
  | { status: 'unsupported' }
  | { status: 'not_available' }
  | { status: 'available'; version: string; body?: string }
  | { status: 'installed'; version: string }

export async function checkAppUpdate(): Promise<AppUpdateOutcome> {
  if (!detectTauriRuntime()) {
    return { status: 'unsupported' }
  }

  const { check } = await import('@tauri-apps/plugin-updater')
  const update = await check()
  if (!update) {
    return { status: 'not_available' }
  }

  return {
    status: 'available',
    version: update.version,
    body: update.body,
  }
}

export async function installAppUpdate(): Promise<AppUpdateOutcome> {
  if (!detectTauriRuntime()) {
    return { status: 'unsupported' }
  }

  const [{ check }, { relaunch }] = await Promise.all([
    import('@tauri-apps/plugin-updater'),
    import('@tauri-apps/plugin-process'),
  ])
  const update = await check()
  if (!update) {
    return { status: 'not_available' }
  }

  await update.downloadAndInstall()
  await relaunch()

  return {
    status: 'installed',
    version: update.version,
  }
}
