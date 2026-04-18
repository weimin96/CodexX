import { invoke } from '@tauri-apps/api/core'
import type {
  Account,
  CreateAccountInput,
  UpdateAccountInput,
  AuthCheckResult,
  UsageSummary,
  ChartDataPoint,
  UsageQuery,
  AppSettings,
  StatusCheckResult,
  LocalAuthSyncResult,
} from '@/types'

type TauriRuntimeWindow = Window & { __TAURI__?: unknown }

// 检测是否在 Tauri 环境中运行
const isTauri = typeof window !== 'undefined' && Boolean((window as TauriRuntimeWindow).__TAURI__)

// Mock 数据，用于非 Tauri 环境（浏览器开发模式）
const mockAccounts: Account[] = []

// ============================================================
// Account Service
// ============================================================

export const accountService = {
  async createAccount(input: CreateAccountInput): Promise<Account> {
    if (!isTauri) return {} as Account
    return invoke<Account>('create_account', { input })
  },

  async updateAccount(input: UpdateAccountInput): Promise<Account> {
    if (!isTauri) return {} as Account
    return invoke<Account>('update_account', { input })
  },

  async deleteAccount(id: string): Promise<void> {
    if (!isTauri) return
    return invoke('delete_account', { id })
  },

  async listAccounts(): Promise<Account[]> {
    if (!isTauri) return mockAccounts
    return invoke<Account[]>('list_accounts')
  },

  async getAccount(id: string): Promise<Account> {
    if (!isTauri) return {} as Account
    return invoke<Account>('get_account', { id })
  },

  async switchAccount(id: string): Promise<void> {
    if (!isTauri) return
    return invoke('switch_account', { id })
  },

  async setDefaultAccount(id: string): Promise<void> {
    if (!isTauri) return
    return invoke('set_default_account', { id })
  },

  async exportAccounts(password: string): Promise<string> {
    if (!isTauri) return ''
    return invoke<string>('export_accounts', { password })
  },

  async importAccounts(encryptedData: string, password: string): Promise<number> {
    if (!isTauri) return 0
    return invoke<number>('import_accounts', { encryptedData, password })
  },

  async syncLocalAuthFile(authFilePath?: string): Promise<LocalAuthSyncResult> {
    if (!isTauri) {
      throw new Error('本地同步仅在 Tauri 环境中可用')
    }
    return invoke<LocalAuthSyncResult>('sync_local_auth_file', { authFilePath })
  },
}

// ============================================================
// Auth Service
// ============================================================

export const authService = {
  async refreshToken(accountId: string): Promise<AuthCheckResult> {
    if (!isTauri) return { account_id: accountId, status: 'unknown', message: '当前不在 Tauri 环境' }
    return invoke<AuthCheckResult>('refresh_token', { accountId })
  },

  async validateToken(accountId: string): Promise<AuthCheckResult> {
    if (!isTauri) return { account_id: accountId, status: 'unknown', message: '当前不在 Tauri 环境' }
    return invoke<AuthCheckResult>('validate_token', { accountId })
  },

  async getAuthStatus(accountId: string): Promise<string> {
    if (!isTauri) return 'unknown'
    return invoke<string>('get_auth_status', { accountId })
  },
}

// ============================================================
// Status Service
// ============================================================

export const statusService = {
  async checkStatus(accountId: string): Promise<StatusCheckResult> {
    if (!isTauri) return { account_id: accountId, status: 'unknown', message: '当前不在 Tauri 环境' }
    return invoke<StatusCheckResult>('check_status', { accountId })
  },

  async checkAllStatus(): Promise<StatusCheckResult[]> {
    if (!isTauri) return []
    return invoke<StatusCheckResult[]>('check_all_status')
  },
}

// ============================================================
// Usage Service
// ============================================================

export const usageService = {
  async fetchUsage(accountId: string): Promise<void> {
    if (!isTauri) return
    return invoke('fetch_usage', { accountId })
  },

  async getUsageStats(query: UsageQuery): Promise<UsageSummary> {
    if (!isTauri) {
      return {
        account_id: query.account_id,
        period: query.period,
        total_input_tokens: 0,
        total_output_tokens: 0,
        total_requests: 0,
        total_cost: 0,
      }
    }
    return invoke<UsageSummary>('get_usage_stats', { query })
  },

  async getUsageChartData(query: UsageQuery): Promise<ChartDataPoint[]> {
    if (!isTauri) return []
    return invoke<ChartDataPoint[]>('get_usage_chart_data', { query })
  },
}

// ============================================================
// Settings Service
// ============================================================

export const settingsService = {
  async getSettings(): Promise<AppSettings> {
    if (!isTauri) {
      return {
        theme: 'dark',
        language: 'zh-CN',
        check_interval: '300',
        autostart: 'false',
        local_auth_auto_sync: 'true',
        local_auth_file_path: '',
      }
    }
    return invoke<AppSettings>('get_settings')
  },

  async saveSettings(settings: Partial<AppSettings>): Promise<void> {
    if (!isTauri) return
    return invoke('save_settings', { settings })
  },

  async setAutostart(enabled: boolean): Promise<void> {
    if (!isTauri) return
    return invoke('set_autostart', { enabled })
  },
}
