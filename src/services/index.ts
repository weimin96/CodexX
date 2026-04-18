import { invoke, isTauri as detectTauriRuntime } from '@tauri-apps/api/core'
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
  PreparedOAuthLogin,
  OAuthLoginResult,
} from '@/types'

// 检测是否在 Tauri 环境中运行
const isTauri = detectTauriRuntime()

// 模拟数据用于浏览器开发模式，避免普通网页预览直接调用本地命令。
const mockAccounts: Account[] = []

// ============================================================
// 账号服务
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

  async syncLocalAuthFile(): Promise<LocalAuthSyncResult> {
    if (!isTauri) {
      throw new Error('本地同步仅在 Tauri 环境中可用')
    }
    return invoke<LocalAuthSyncResult>('sync_local_auth_file', { authFilePath: null })
  },
}

// ============================================================
// 认证服务
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

  async prepareOAuthLogin(): Promise<PreparedOAuthLogin> {
    if (!isTauri) {
      throw new Error('OAuth 网页登录仅在 Tauri 环境中可用')
    }
    return invoke<PreparedOAuthLogin>('prepare_oauth_login')
  },

  async openOAuthLoginUrl(url: string): Promise<void> {
    if (!isTauri) {
      throw new Error('OAuth 网页登录仅在 Tauri 环境中可用')
    }
    return invoke('open_oauth_login_url', { url })
  },

  async completeOAuthCallbackLogin(callbackUrl: string): Promise<OAuthLoginResult> {
    if (!isTauri) {
      throw new Error('OAuth 网页登录仅在 Tauri 环境中可用')
    }
    return invoke<OAuthLoginResult>('complete_oauth_callback_login', { callbackUrl })
  },

  async cancelOAuthLogin(): Promise<void> {
    if (!isTauri) return
    return invoke('cancel_oauth_login')
  },
}

// ============================================================
// 状态服务
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
// 用量服务
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
// 设置服务
// ============================================================

export const settingsService = {
  async getSettings(): Promise<AppSettings> {
    if (!isTauri) {
      return {
        theme: 'dark',
        language: 'zh-CN',
        check_interval: '300',
        autostart: 'false',
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
