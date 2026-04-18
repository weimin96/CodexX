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
} from '@/types'

// ============================================================
// Account Service
// ============================================================

export const accountService = {
  async createAccount(input: CreateAccountInput): Promise<Account> {
    return invoke<Account>('create_account', { input })
  },

  async updateAccount(input: UpdateAccountInput): Promise<Account> {
    return invoke<Account>('update_account', { input })
  },

  async deleteAccount(id: string): Promise<void> {
    return invoke('delete_account', { id })
  },

  async listAccounts(): Promise<Account[]> {
    return invoke<Account[]>('list_accounts')
  },

  async getAccount(id: string): Promise<Account> {
    return invoke<Account>('get_account', { id })
  },

  async switchAccount(id: string): Promise<void> {
    return invoke('switch_account', { id })
  },

  async setDefaultAccount(id: string): Promise<void> {
    return invoke('set_default_account', { id })
  },

  async exportAccounts(password: string): Promise<string> {
    return invoke<string>('export_accounts', { password })
  },

  async importAccounts(encryptedData: string, password: string): Promise<number> {
    return invoke<number>('import_accounts', { encryptedData, password })
  },
}

// ============================================================
// Auth Service
// ============================================================

export const authService = {
  async refreshToken(accountId: string): Promise<AuthCheckResult> {
    return invoke<AuthCheckResult>('refresh_token', { accountId })
  },

  async validateToken(accountId: string): Promise<AuthCheckResult> {
    return invoke<AuthCheckResult>('validate_token', { accountId })
  },

  async getAuthStatus(accountId: string): Promise<string> {
    return invoke<string>('get_auth_status', { accountId })
  },
}

// ============================================================
// Status Service
// ============================================================

export const statusService = {
  async checkStatus(accountId: string): Promise<StatusCheckResult> {
    return invoke<StatusCheckResult>('check_status', { accountId })
  },

  async checkAllStatus(): Promise<StatusCheckResult[]> {
    return invoke<StatusCheckResult[]>('check_all_status')
  },
}

// ============================================================
// Usage Service
// ============================================================

export const usageService = {
  async fetchUsage(accountId: string): Promise<void> {
    return invoke('fetch_usage', { accountId })
  },

  async getUsageStats(query: UsageQuery): Promise<UsageSummary> {
    return invoke<UsageSummary>('get_usage_stats', { query })
  },

  async getUsageChartData(query: UsageQuery): Promise<ChartDataPoint[]> {
    return invoke<ChartDataPoint[]>('get_usage_chart_data', { query })
  },
}

// ============================================================
// Settings Service
// ============================================================

export const settingsService = {
  async getSettings(): Promise<AppSettings> {
    return invoke<AppSettings>('get_settings')
  },

  async saveSettings(settings: Partial<AppSettings>): Promise<void> {
    return invoke('save_settings', { settings })
  },

  async setAutostart(enabled: boolean): Promise<void> {
    return invoke('set_autostart', { enabled })
  },
}
