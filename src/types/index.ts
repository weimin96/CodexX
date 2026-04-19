// ============================================================
// 账号类型
// ============================================================

export type AuthType = 'api_key' | 'oauth_token' | 'cookie_session' | 'cli_profile'

export type AccountStatus = 'normal' | 'warning' | 'error' | 'unknown' | 'expired'

export interface Account {
  id: string
  name: string
  auth_type: AuthType
  email?: string
  organization?: string
  is_default: boolean
  is_active: boolean
  created_at: string
  updated_at: string
  last_checked_at?: string
  status: AccountStatus
  status_message?: string
  color: string
  avatar_text?: string
  codex_plan_type?: string
  codex_usage_fetched_at?: string
  codex_usage_5h?: CodexUsageWindow
  codex_usage_week?: CodexUsageWindow
  codex_usage_error?: string
}

export interface CodexUsageWindow {
  used_percent: number
  window_seconds: number
  reset_at?: number
}

export interface CreateAccountInput {
  name?: string
  auth_type: AuthType
  email?: string
  organization?: string
  color?: string
  credential_value: string
  credential_type?: string
}

export interface UpdateAccountInput {
  id: string
  name?: string
  email?: string
  organization?: string
  color?: string
  credential_value?: string
}

// ============================================================
// 认证类型
// ============================================================

export type AuthStatus = 'valid' | 'expired' | 'invalid' | 'unknown'

export interface AuthCheckResult {
  account_id: string
  status: AuthStatus
  message?: string
  expires_at?: string
}

export interface PreparedOAuthLogin {
  auth_url: string
  redirect_uri: string
}

export interface OAuthLoginResult {
  account_id: string
  account_name: string
  auth_type: AuthType
}

export interface OAuthCallbackFinishedEvent {
  result: OAuthLoginResult | null
  error: string | null
}

// ============================================================
// 用量类型
// ============================================================

export interface UsageSummary {
  account_id: string
  period: string
  total_input_tokens: number
  total_output_tokens: number
  total_requests: number
  total_cost: number
}

export interface ChartDataPoint {
  date: string
  input_tokens: number
  output_tokens: number
  request_count: number
  cost: number
}

export type UsagePeriod = 'day' | 'week' | 'month'

export interface UsageQuery {
  account_id: string
  period: UsagePeriod
}

export interface CodexExecInput {
  account_id: string
  prompt: string
  working_directory?: string
  model?: string
  profile?: string
  sandbox?: string
  config_overrides?: string[]
  skip_git_repo_check?: boolean
}

export interface CodexInteractiveInput {
  account_id: string
  prompt?: string
  working_directory?: string
  model?: string
  profile?: string
  sandbox?: string
}

export interface CodexCliLaunchInput {
  account_id?: string
  working_directory?: string
  model?: string
}

export interface CodexAppLaunchInput {
  account_id?: string
}

export interface UsageImportResult {
  account_id: string
  session_count: number
  scanned_file_count: number
  imported_count: number
  ignored_line_count: number
}

export interface CodexModelOption {
  label: string
  value: string
}

export interface CodexLauncherConfig {
  default_model?: string
  model_options: CodexModelOption[]
}

export interface CodexConfigEntry {
  key: string
  value_type: string
  value: string
}

export interface CodexConfigSnapshot {
  path: string
  exists: boolean
  raw_text: string
  parsed_entries: CodexConfigEntry[]
  backup_path?: string
}

export interface CodexLaunchResult {
  session_id: string
  status: string
  exit_code?: number
  usage_event_count: number
  message: string
  stderr_preview?: string
}

export interface CodexShortConversationResult extends CodexLaunchResult {
  account_id: string
  account_name: string
  model: string
}

// ============================================================
// 设置类型
// ============================================================

export interface AppSettings {
  theme: 'dark' | 'light'
  language: string
  check_interval: string
  autostart: string
  token_keepalive_enabled: string
  quota_alert_enabled: string
  auto_update_enabled: string
}

export interface CodexQuotaExhaustedEvent {
  account_id: string
  account_name: string
  plan_type?: string
  five_hour_used_percent?: number
  weekly_used_percent?: number
  task_label: string
}

// ============================================================
// 状态类型
// ============================================================

export interface StatusCheckResult {
  account_id: string
  status: AccountStatus
  message?: string
}

export interface LocalAuthSyncResult {
  account_id: string
  account_name: string
  auth_type: AuthType
  auth_file_path: string
  codex_plan_type?: string
  codex_usage_error?: string
}

export interface LocalDefaultAccountSyncResult {
  matched_account_id?: string | null
  updated: boolean
  skipped_reason?: string | null
}

export interface AccountExportResult {
  exported_count: number
  failed_count: number
  output_path: string
  errors: string[]
}

export interface AccountImportResult {
  imported_count: number
  skipped_count: number
  failed_count: number
  account_ids: string[]
  errors: string[]
}

// ============================================================
// 界面辅助类型
// ============================================================

export const AUTH_TYPE_LABELS: Record<AuthType, string> = {
  api_key: 'API Key',
  oauth_token: 'OAuth Token',
  cookie_session: 'Cookie / Session',
  cli_profile: 'CLI Profile',
}

export const STATUS_LABELS: Record<AccountStatus, string> = {
  normal: '正常',
  warning: '警告',
  error: '异常',
  unknown: '待确认',
  expired: '已过期',
}

export const STATUS_COLORS: Record<AccountStatus, string> = {
  normal: '#18a058',
  warning: '#f0a020',
  error: '#d03050',
  unknown: '#909399',
  expired: '#8b5cf6',
}
