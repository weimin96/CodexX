# CodexX

<p align="center">
  <img src="docs/readme/codexx-logo.png" alt="CodexX Logo" width="144" />
</p>

<p align="center">
  <img src="docs/readme/codexx-overview.png" alt="CodexX 项目概览图" />
</p>

CodexX 是一个基于 Tauri 2、Vue 3 和 Rust 的桌面应用，用于管理 Codex 账号、本地认证文件、Codex 配置和本机用量统计。

## 功能概览

### 账号与凭证

- 支持 API Key、OAuth Token、Cookie Session、CLI Profile 四类账号。
- 数据库凭证加密存储，数据库默认位于用户目录 `.codex\CodexManager\codex.db`。该路径为历史兼容路径，重命名为 CodexX 后继续沿用，避免既有数据迁移风险。
- 切换账号会把对应凭证写回默认 `auth.json`，写入前生成 `auth.json.bak`。
- 账号卡片可导出标准 `auth.json`。
- 账号操作可批量导出 zip，包内每个账号一个独立 JSON 文件。
- 导入支持单个 `auth.json` 和上述 zip 包，并按账号身份去重更新。

### Codex 启动与用量

- 启动器支持打开 Codex CLI 和 Codex App。
- 通过 CodexX 显式启动的 CLI/App 会话会记录账号关联，用量页刷新时从本机 `.codex\sessions` JSONL 导入 `usage` Token 统计。
- 受控 `codex exec` 和一键预热任务会记录逐次 Token 用量。
- 账号操作菜单提供“一键预热”，只会选择 5 小时剩余额度为 100% 的账号，并使用 `GPT-5.3-Codex` 与低推理配置执行最短对话。
- 可在设置页开启额度用尽提醒；受控任务完成后，如果 5 小时或 7 天 Codex 额度用尽，会提示切换账号。

### Codex 配置

- 提供 Codex 配置页面，读取用户级 `.codex\config.toml`。
- 页面以表单列出 Codex 官方配置字段、可选项和字段说明。
- 修改单个字段后立即保存该字段，后端只替换目标字段赋值行，不重新序列化整份 TOML，尽量保留其它字段、顺序和注释。
- 动态字段需要先填写实际字段名，例如 `mcp_servers.local.command`。

### 设置与系统能力

- 支持浅色和深色主题。
- 支持开机自启。
- 支持后台状态检测间隔配置。
- 支持 OAuth Token 定期保活；启用后按最小间隔刷新 `access_token` 和 `refresh_token`，并回写加密数据库。
- 支持手动检查更新和启动时自动检查更新。稳定版默认通过 GitHub Releases `latest.json` 检查并下载新版本。

## 安全设计

### 主密钥来源

凭证加密使用 AES-256-GCM。主密钥按以下优先级获取：

- 如果进程环境变量 `CODEX_MANAGER_MASTER_KEY` 存在，使用该值作为显式覆盖主密钥。
- 如果未设置环境变量，使用系统凭据库保存或读取主密钥；Windows 使用 Credential Manager，macOS 使用 Keychain。

应用不会自动读取 `.env` 文件。开发和自动化场景如需固定主密钥，应由启动脚本或 shell 把环境变量注入当前进程。

`CODEX_MANAGER_MASTER_KEY` 支持以下格式：

- 32 字节原文。
- 64 位十六进制字符串。
- base64 编码的 32 字节值。

Windows PowerShell 当前会话示例：

```powershell
$env:CODEX_MANAGER_MASTER_KEY = "<32字节主密钥>"
pnpm tauri dev
```

Windows 用户级环境变量示例：

```powershell
[Environment]::SetEnvironmentVariable("CODEX_MANAGER_MASTER_KEY", "<32字节主密钥>", "User")
```

### 导出文件

导出的 `auth.json` 和 zip 包按 Codex 标准认证文件格式保存，文件内包含明文 Token 或 API Key。导出文件需要按敏感凭证保管，不应提交到仓库、日志或聊天记录。

## 快速开始

### 前置要求

| 工具 | 说明 |
| --- | --- |
| Node.js 18 或更高版本 | 前端依赖与构建 |
| pnpm 8 或更高版本 | 包管理 |
| Rust 1.77 或更高版本 | Tauri 后端编译 |
| Visual Studio Build Tools | Windows C++ 工具链 |
| WebView2 | Windows 运行时 |

### Windows 开发命令

```powershell
pnpm install
pnpm tauri dev
```

### 常用验证命令

```powershell
cargo check --manifest-path src-tauri\Cargo.toml --no-default-features
npm run build
git diff --check
```

针对 Codex 配置页单字段保存：

```powershell
cargo test --manifest-path src-tauri\Cargo.toml codex_config --no-default-features
```

针对 OAuth Token 保活：

```powershell
cargo test --manifest-path src-tauri\Cargo.toml token_refresh_tests --no-default-features
```

针对 Codex 会话用量导入：

```powershell
cargo test --manifest-path src-tauri\Cargo.toml codex_session_import --no-default-features
```

## 项目结构

```text
codexx
├── src
│   ├── components
│   ├── router
│   ├── services
│   ├── stores
│   ├── types
│   ├── utils
│   └── views
├── src-tauri
│   ├── capabilities
│   ├── icons
│   └── src
│       ├── account
│       ├── auth
│       ├── codex_config.rs
│       ├── codex_runtime
│       ├── codex_session_import.rs
│       ├── codex_usage
│       ├── commands
│       ├── scheduler
│       ├── security
│       ├── status_sync
│       ├── storage
│       └── usage
├── package.json
├── tsconfig.json
└── vite.config.ts
```

## 后端命令边界

前端通过 `src\services\index.ts` 调用 Tauri command。主要命令包括：

| 模块 | 命令 |
| --- | --- |
| 账号 | `list_accounts`、`get_account`、`create_account`、`update_account`、`delete_account`、`switch_account` |
| 导入导出 | `export_account_auth_file`、`export_accounts`、`import_accounts` |
| 认证 | `prepare_oauth_login`、`complete_oauth_callback_login`、`refresh_token`、`validate_token` |
| 状态 | `check_status`、`check_all_status` |
| 用量 | `fetch_usage`、`get_usage_stats`、`get_usage_chart_data` |
| Codex 启动 | `launch_codex_cli`、`launch_codex_app`、`trigger_codex_short_conversation` |
| Codex 配置 | `read_codex_config_file`、`save_codex_config_field`、`save_codex_config_file` |
| 设置 | `get_settings`、`save_settings`、`set_autostart` |

## 更新配置

Tauri updater 已接入前端手动检查和启动自动检查。当前仓库默认使用 GitHub Releases 作为稳定更新入口：

```json
{
  "bundle": {
    "createUpdaterArtifacts": true
  },
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://github.com/weimin96/CodexX/releases/latest/download/latest.json"
      ],
      "pubkey": "仓库当前生成的 updater 公钥",
      "windows": {
        "installMode": "passive"
      }
    }
  }
}
```

`pubkey` 需要与 GitHub Actions 中用于签名 updater 产物的私钥配套。私钥不要提交到仓库。

GitHub Releases 方案有一个前提：`latest.json` 和安装包地址必须能被客户端匿名访问。如果仓库是私有仓库，GitHub 会对未登录请求返回 `404`，Tauri updater 无法使用该地址完成真实终端更新。

## 发布稳定版

### 必备仓库 Secret

- `TAURI_SIGNING_PRIVATE_KEY`：updater 私钥内容。
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：私钥密码；无密码时可留空或不设置。

### 触发方式

- 向远程仓库推送 `v*` 格式 tag，例如 `v0.1.2`。
- GitHub Actions 工作流 `Tag 发布` 会构建安装包、签名 updater 产物并上传 `latest.json`。
- 应用内稳定更新地址固定为：

```text
https://github.com/weimin96/CodexX/releases/latest/download/latest.json
```

### 本地生成签名密钥

Windows PowerShell 示例：

```powershell
pnpm exec tauri signer generate -- --ci -w "$env:USERPROFILE\.tauri\codexx-updater.key"
```

生成后：

- 私钥文件默认位于 `C:\Users\<用户名>\.tauri\codexx-updater.key`
- 公钥文件默认位于 `C:\Users\<用户名>\.tauri\codexx-updater.key.pub`
- GitHub Actions 需要读取私钥内容作为 `TAURI_SIGNING_PRIVATE_KEY`

## 重要边界

- CodexX 不做透明网络代理，也不全局拦截外部 Codex 进程。
- Codex CLI/App 用量统计只覆盖通过 CodexX 显式启动并能在本机 `.codex\sessions` 中找到 usage 记录的会话。
- OpenAI 官方 API Usage 端点是组织级 API 用量，不等同于 ChatGPT 计划下的本地 Codex 额度。
- 只有最新正式版 GitHub Release 会被 `releases/latest` 解析；draft 和 prerelease 不会作为稳定更新源。
- 当前仓库如果保持 `private`，GitHub Releases 只适合作为内部发布资产存储，不适合作为面向终端用户的 updater 源。要让应用内在线更新真正可用，至少需要公开仓库，或把 `latest.json` 与安装包迁移到公开可访问地址。
