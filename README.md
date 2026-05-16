# CodexX

<p align="center">
  <img src="docs/readme/codexx-overview.png" alt="CodexX 项目概览图" />
</p>

CodexX 是一个基于 Tauri 2、Vue 3 和 Rust 的桌面应用，用于管理 Codex 账号、本地认证文件、Codex 配置、本机用量统计和 Codex 启动入口。

## 功能概览

### 账号与凭证

- 支持 API Key、OAuth Token、本地 `auth.json` 同步和账号导入导出。
- 展示 5 小时与 7 天 Codex 额度窗口、剩余额度和重置时间。
- 支持账号切换，并将可写回凭证同步到本机默认 `auth.json`。
- 提供周期预热，通过一次最短 Codex 对话触发周期额度开始计时。
- 支持 OAuth Token 保活，按设置的后台检测周期检查账号状态，并在 Token 需要刷新时更新本地凭证。

### 用量统计

- 汇总本机 Codex 启动会话和 API 用量事件。
- 展示输入 Token、缓存命中 Token、输出 Token、请求次数和估算费用。
- 支持按账号重建历史用量，避免相似账号或历史导入错误长期污染统计结果。

### Codex 配置与启动

- 读取和保存用户级 `~/.codex/config.toml`。
- 支持受控 `codex exec --json`、交互式 Codex CLI、Codex CLI 终端和 Codex App 启停。
- Codex App 启停当前仅支持 Windows。

## 安装

从 [GitHub Releases](https://github.com/weimin96/CodexX/releases) 下载与系统匹配的安装包。

| 平台 | 安装包 |
| --- | --- |
| Windows | 下载 `.msi` 或 `.exe` |
| macOS Intel | 下载 `x64` 的 `.dmg` 或 `.app.tar.gz` |
| macOS Apple Silicon | 下载 `aarch64` 的 `.dmg` 或 `.app.tar.gz` |

Release 附带 updater 签名产物；新增发布会附带 `SHA256SUMS`，可用于手工校验安装包完整性。

## 数据位置

| 数据 | 位置 |
| --- | --- |
| 数据库 | `~/.codex/CodexX/codexX.db` |
| Codex 配置 | `~/.codex/config.toml` |
| 本地认证文件 | `~/.codex/auth.json` |
| 默认认证备份 | `~/.codex/auth.json.bak` |

Windows 下 `~` 对应 `%USERPROFILE%`，例如 `C:\Users\<用户名>\.codex\CodexX\codexX.db`。

## 安全说明

- 账号凭证会使用 AES-256-GCM 加密后写入 SQLite 数据库。
- 主密钥优先读取 `CODEX_MANAGER_MASTER_KEY`；未设置时使用系统凭据库，Windows 为 Credential Manager，macOS 为 Keychain。
- Token 保活默认关闭。启用后，后台任务按设置页的检测间隔运行，默认 300 秒；OAuth Token 距上次刷新不足 30 分钟时不会重复刷新。
- 周期预热会先把目标账号写入默认 `auth.json`，再执行一次 `codex exec --json` 最短对话，当前提示词为 `hi`，模型为 `GPT-5.2`，sandbox 为 `read-only`。
- 周期预热会产生真实 Codex 请求和用量事件，适合用于触发额度周期计时，不应用作无成本检测。
- 导出的 `auth.json` 或账号压缩包包含明文凭证，应按敏感文件保管。
- 删除账号只会删除 CodexX 数据库中的账号和加密凭证；如需彻底清理本机默认认证文件和数据库，需要同时删除 `~/.codex/auth.json`、`~/.codex/auth.json.bak` 和 `~/.codex/CodexX`。

更多用户侧操作说明见 [用户文档](./docs/user-guide.md)。
发布完整性与供应链门禁说明见 [发布可信度说明](./docs/release-hardening.md)。

## 常见问题

### 为什么额度显示不同步

CodexX 只能展示本机已同步到的账号资料和本机导入的会话用量。若你在其他设备或其他 Codex 客户端中使用账号，额度可能需要重新检测账号状态或重建历史用量后才会接近实际情况。

### OAuth 登录失败

OAuth 登录需要系统浏览器访问 `https://auth.openai.com`，应用只监听 `127.0.0.1` 本地回调。若登录失败，请重新打开 OAuth 登录流程，避免复用过期回调链接。

### Codex App 无法启动

Codex App 启停当前仅支持 Windows，并依赖本机已经安装 Codex App。macOS 用户仍可使用账号、配置、用量统计和安装包更新能力。

### 自动更新失败

自动更新依赖 GitHub Releases 的 `latest.json`、updater 签名和网络访问。私有仓库、代理拦截、签名不匹配或发布资产缺失都会导致更新失败。失败时可以从 Releases 手动下载安装包，并用 `SHA256SUMS` 校验完整性。

## 开发

### 前置要求

| 工具 | 说明 |
| --- | --- |
| Node.js 22 | 前端依赖与构建，CI 使用该版本 |
| pnpm 9.15.9 | 包管理，CI 使用该版本 |
| Rust stable | Tauri 后端编译 |
| Visual Studio Build Tools | Windows C++ 工具链 |
| WebView2 | Windows 运行时 |

### Windows 开发命令

```powershell
pnpm install
pnpm tauri dev
```

### 最小验证命令

```powershell
pnpm build
cargo check --manifest-path src-tauri\Cargo.toml --no-default-features
git diff --check
```

## 开源协议

本项目基于 Apache-2.0 协议开源，完整协议文本见 [LICENSE](./LICENSE)。
