import { invoke, isTauri as detectTauriRuntime } from '@tauri-apps/api/core'
import type {
  Account,
  CreateAccountInput,
  UpdateAccountInput,
  AuthCheckResult,
  UsageSummary,
  ChartDataPoint,
  CodexAppCloseResult,
  CodexAppLaunchInput,
  UsageQuery,
  UsageImportResult,
  CodexCliLaunchInput,
  CodexConfigSnapshot,
  CodexConfigFieldUpdate,
  CodexLauncherConfig,
  CodexExecInput,
  CodexInteractiveInput,
  CodexLaunchResult,
  CodexShortConversationResult,
  AppSettings,
  StatusCheckResult,
  LocalAuthSyncResult,
  LocalDefaultAccountSyncResult,
  AccountExportResult,
  AccountImportResult,
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

  async getAccountCredential(id: string): Promise<string> {
    if (!isTauri) return ''
    return invoke<string>('get_account_credential', { id })
  },

  async switchAccount(id: string): Promise<void> {
    if (!isTauri) return
    return invoke('switch_account', { id })
  },

  async setDefaultAccount(id: string): Promise<void> {
    if (!isTauri) return
    return invoke('set_default_account', { id })
  },

  async exportAccountAuthFile(accountId: string, outputPath: string): Promise<AccountExportResult> {
    if (!isTauri) {
      return { exported_count: 0, failed_count: 0, output_path: outputPath, errors: [] }
    }
    return invoke<AccountExportResult>('export_account_auth_file', { accountId, outputPath })
  },

  async exportAccounts(outputPath: string): Promise<AccountExportResult> {
    if (!isTauri) {
      return { exported_count: 0, failed_count: 0, output_path: outputPath, errors: [] }
    }
    return invoke<AccountExportResult>('export_accounts', { outputPath })
  },

  async importAccounts(inputPath: string): Promise<AccountImportResult> {
    if (!isTauri) {
      return { imported_count: 0, skipped_count: 0, failed_count: 0, account_ids: [], errors: [] }
    }
    return invoke<AccountImportResult>('import_accounts', { inputPath })
  },

  async syncLocalAuthFile(): Promise<LocalAuthSyncResult> {
    if (!isTauri) {
      throw new Error('本地同步仅在 Tauri 环境中可用')
    }
    return invoke<LocalAuthSyncResult>('sync_local_auth_file', { authFilePath: null })
  },

  async syncLocalDefaultAccount(): Promise<LocalDefaultAccountSyncResult> {
    if (!isTauri) {
      return { updated: false, skipped_reason: '当前不在 Tauri 环境' }
    }
    return invoke<LocalDefaultAccountSyncResult>('sync_local_default_account')
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
  async fetchUsage(accountId: string): Promise<UsageImportResult> {
    if (!isTauri) {
      return {
        account_id: accountId,
        session_count: 0,
        scanned_file_count: 0,
        imported_count: 0,
        ignored_line_count: 0,
      }
    }
    return invoke<UsageImportResult>('fetch_usage', { accountId })
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

  async runCodexExec(input: CodexExecInput): Promise<CodexLaunchResult> {
    if (!isTauri) {
      throw new Error('受控启动 Codex 仅在 Tauri 环境中可用')
    }
    return invoke<CodexLaunchResult>('run_codex_exec_session', { input })
  },

  async triggerCodexShortConversation(accountId?: string): Promise<CodexShortConversationResult> {
    if (!isTauri) {
      throw new Error('触发 Codex 对话仅在 Tauri 环境中可用')
    }
    return invoke<CodexShortConversationResult>('trigger_codex_short_conversation', {
      accountId: accountId ?? null,
      warmupWindow: 'five_hour',
    })
  },

  async triggerCodexWarmupConversation(
    warmupWindow: 'five_hour' | 'one_week',
    accountId?: string,
  ): Promise<CodexShortConversationResult> {
    if (!isTauri) {
      throw new Error('触发 Codex 对话仅在 Tauri 环境中可用')
    }
    return invoke<CodexShortConversationResult>('trigger_codex_short_conversation', {
      accountId: accountId ?? null,
      warmupWindow,
    })
  },

  async openCodexInteractive(input: CodexInteractiveInput): Promise<CodexLaunchResult> {
    if (!isTauri) {
      throw new Error('受控启动 Codex 仅在 Tauri 环境中可用')
    }
    return invoke<CodexLaunchResult>('open_codex_interactive_session', { input })
  },

  async launchCodexCli(input: CodexCliLaunchInput): Promise<CodexLaunchResult> {
    if (!isTauri) {
      throw new Error('受控启动 Codex 仅在 Tauri 环境中可用')
    }
    return invoke<CodexLaunchResult>('launch_codex_cli', { input })
  },

  async getCodexLauncherConfig(): Promise<CodexLauncherConfig> {
    if (!isTauri) {
      return {
        default_model: undefined,
        model_options: [],
        trusted_project_paths: [],
      }
    }
    return invoke<CodexLauncherConfig>('get_codex_launcher_config')
  },

  async launchCodexApp(input: CodexAppLaunchInput = {}): Promise<CodexLaunchResult> {
    if (!isTauri) {
      throw new Error('受控启动 Codex 仅在 Tauri 环境中可用')
    }
    return invoke<CodexLaunchResult>('launch_codex_app', { input })
  },

  async closeCodexApp(): Promise<CodexAppCloseResult> {
    if (!isTauri) {
      throw new Error('关闭 Codex App 仅在 Tauri 环境中可用')
    }
    return invoke<CodexAppCloseResult>('close_codex_app')
  },

  async clearUsageData(): Promise<void> {
    if (!isTauri) return
    return invoke('clear_usage_data')
  },
}

// ============================================================
// Codex 配置服务
// ============================================================

export const codexConfigService = {
  async readConfig(): Promise<CodexConfigSnapshot> {
    if (!isTauri) {
      return {
        path: '~/.codex/config.toml',
        exists: false,
        raw_text: '',
        parsed_entries: [],
      }
    }
    return invoke<CodexConfigSnapshot>('read_codex_config_file')
  },

  async saveConfig(rawText: string): Promise<CodexConfigSnapshot> {
    if (!isTauri) {
      return {
        path: '~/.codex/config.toml',
        exists: true,
        raw_text: rawText,
        parsed_entries: [],
      }
    }
    return invoke<CodexConfigSnapshot>('save_codex_config_file', { rawText })
  },

  async saveConfigField(input: CodexConfigFieldUpdate): Promise<CodexConfigSnapshot> {
    if (!isTauri) {
      return {
        path: '~/.codex/config.toml',
        exists: true,
        raw_text: `${input.key} = ${input.value}\n`,
        parsed_entries: [
          {
            key: input.key,
            value_type: 'unknown',
            value: input.value,
          },
        ],
      }
    }
    return invoke<CodexConfigSnapshot>('save_codex_config_field', { input })
  },
}

// ============================================================
// 设置服务
// ============================================================

export const settingsService = {
  async getSettings(): Promise<AppSettings> {
    if (!isTauri) {
      return {
        theme: 'light',
        language: 'zh-CN',
        check_interval: '300',
        autostart: 'false',
        token_keepalive_enabled: 'false',
        auto_update_enabled: 'false',
        window_close_action: 'tray',
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
