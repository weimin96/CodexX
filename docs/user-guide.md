# CodexX 用户文档

## 安装与更新

从 [GitHub Releases](https://github.com/weimin96/CodexX/releases) 下载安装包。

| 平台 | 文件 |
| --- | --- |
| Windows | `.msi` 或 `.exe` |
| macOS Intel | `x64` `.dmg` 或 `.app.tar.gz` |
| macOS Apple Silicon | `aarch64` `.dmg` 或 `.app.tar.gz` |

自动更新使用 Tauri updater，安装包需要通过发布时的签名校验。Release 中的 `SHA256SUMS` 用于手工校验下载文件是否完整。

## 数据位置

| 数据 | 默认位置 |
| --- | --- |
| CodexX 数据库 | `~/.codex/CodexX/codexX.db` |
| Codex 配置 | `~/.codex/config.toml` |
| 本地认证文件 | `~/.codex/auth.json` |
| 写回前备份 | `~/.codex/auth.json.bak` |

Windows 下 `~` 是 `%USERPROFILE%`。macOS 下 `~` 是当前用户主目录。

## 账号与凭证

CodexX 支持 API Key、OAuth Token、Cookie Session 和 CLI Profile 账号类型。API Key 与 OAuth Token 可以写回标准 Codex `auth.json`；Cookie Session 与 CLI Profile 不会写回标准 `auth.json`。

凭证写入数据库前会加密。主密钥优先读取 `CODEX_MANAGER_MASTER_KEY`，未设置时使用系统凭据库。导出的账号文件包含明文凭证，应单独保管，不应提交到 Git 仓库或聊天记录。

## Token 保活

Token 保活默认关闭。启用后，后台任务会按设置页的检测间隔运行，默认 300 秒。只有 OAuth Token 距上次刷新达到 30 分钟后，应用才会尝试刷新；刷新失败会把账号状态标记为警告并保留错误信息。

如果该账号是当前默认账号，刷新后的凭证会同步写回 `~/.codex/auth.json`。

## 周期预热

周期预热用于触发 Codex 额度周期开始计时。执行时应用会：

- 把目标账号临时写入默认 `auth.json`。
- 调用 `codex exec --json` 发起最短对话。
- 使用提示词 `hi`、模型 `GPT-5.2`、`read-only` sandbox 和低推理配置。
- 结束后刷新账号额度状态。
- 批量预热完成后恢复开始前的默认账号。

周期预热不再要求 5 小时或 7 天剩余额度必须为 100%。只要任一周期窗口尚未进入倒计时，账号就可以执行周期预热。该操作会产生真实请求和用量记录。

## 删除账号与本地数据

在应用内删除账号会删除 CodexX 数据库中的账号和加密凭证，不会自动删除本机默认 Codex 认证文件。

彻底删除本地数据需要关闭应用后处理以下位置：

- `~/.codex/CodexX`
- `~/.codex/auth.json`
- `~/.codex/auth.json.bak`

如果仍需保留 Codex CLI 的当前登录状态，不要删除 `auth.json`。

## 常见问题

### 额度显示不同步

额度资料来自 Codex 账号状态接口和本机已导入的会话日志。其他设备或其他客户端产生的使用量不会立即出现在本机统计中。可以先检测账号状态，再按账号重建历史用量。

### OAuth 登录失败

OAuth 登录会打开 `https://auth.openai.com`，并监听本机 `127.0.0.1` 回调。失败时重新打开登录流程，不要复用旧回调链接。若浏览器或代理拦截本地回调，请允许 `localhost` 回跳到 CodexX。

### Codex App 无法启动

Codex App 启停当前仅支持 Windows，并要求本机已经安装 Codex App。若启动失败，可以改用 Codex CLI 终端或交互式 Codex CLI。

### 自动更新失败

自动更新依赖 GitHub Releases、`latest.json`、updater 签名和网络访问。私有仓库匿名访问、代理拦截、签名不匹配或 Release 资产缺失都会导致失败。可以手动下载对应平台安装包，并用 Release 中的 `SHA256SUMS` 校验。

### 数据库无法打开

应用优先使用 `~/.codex/CodexX/codexX.db`。如果主目录不可写，会回退到系统应用数据目录并记录日志。若数据库损坏，应用会尝试备份损坏文件并创建新库，原文件保留用于后续人工恢复。
