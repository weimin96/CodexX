# AGENTS

## 交付定位

- 本文件是 CodexX 仓库的协作规范，面向会直接修改代码、文档、配置和发布资产的工程代理。
- 任何改动都应被视为可交付工程成果，而不是示例内容。
- 实现前必须先理解现有代码和数据边界，不能只根据文件名或主观猜测修改。
- 交付结果应能被代码审查者复现、验证和追责。

## 工作环境

- 默认操作系统为 Windows，默认终端为 PowerShell。
- 命令、脚本和文档示例优先使用 PowerShell 语法，不使用只适用于 Linux 的语法。
- 可以使用 `rg` 和 `rg --files` 进行搜索。
- 禁止使用 Windows 旧式空设备重定向写法。
- `>/dev/null` 是类 Unix 重定向写法，不是本项目默认 PowerShell 等价方案；在 PowerShell 中需要静默输出时，使用 `$null = <命令>`、管道到 `Out-Null`，或使用命令自身的安静参数，并在跨平台脚本中显式说明差异。
- 所有新增或修改的文件必须使用 UTF-8 编码，不使用 BOM。
- 代码注释、文档、提交说明和交付说明使用中文；技术专有名词、命令、文件名、类型名和协议名可以保留原文。
- 严禁使用 emoji。

## 基本流程

- 先建模，再写代码。
- 建模至少明确核心对象、状态变化、约束条件和失败路径。
- 先阅读相关实现，再做修改；不得在未确认调用链和数据结构的情况下直接替换代码。
- 复杂任务或预计超过五个步骤的任务，必须先更新 `task_plan.md`、`findings.md` 或 `progress.md`，再实施。
- 每完成一个阶段，及时记录关键发现、失败和验证结果。
- 一个独立问题对应一个独立提交；不要把无关修改混入同一次提交。
- 工作区可能已有用户改动，禁止回滚、覆盖或格式化与本任务无关的内容。
- 如果遇到需求歧义、跨模块连锁修改、安全或隐私风险、缺少可执行验证路径，必须停止实现并向用户确认。

## 设计规则

- 命名使用业务领域语言，避免 `temp`、`foo`、`test` 这类无意义命名。
- 函数保持单一职责，一个函数只处理一个抽象层次。
- 只有存在明确变化场景时，才引入设计模式、抽象层或通用框架。
- 优先使用组合，不为了复用少量代码引入继承或复杂泛化。
- 所有失败路径必须显式处理，不吞异常，不静默失败。
- 注释只解释为什么这样设计、有什么约束和取舍；不要写只复述代码动作的注释。
- 修改涉及凭证、账号切换、导入导出、更新器、自动任务、后台调度、文件写入和进程启动时，必须明确失败后的状态和恢复路径。

## 文档规则

- 文档标题使用语义化层级，不使用自动编号或手动编号。
- 生成或修改 docx 文档时，标题不得包含 `1.1`、`一、`、`（1）` 这类编号。
- 文档示例不得包含真实 Token、API Key、refresh token、完整 `auth.json`、主密钥或用户私有路径。
- 涉及敏感文件的说明必须标明敏感属性和保管要求。

## 项目概览

- CodexX 是基于 Tauri 2、Vue 3、TypeScript 和 Rust 的桌面应用。
- 前端使用 Vite、Pinia、Vue Router、Naive UI、ECharts 和 Tauri 插件能力。
- 后端使用 Rust 2021、rusqlite、AES-256-GCM、reqwest、tokio、keyring、zip 和 Tauri 2。
- 当前应用版本在 `package.json`、`src-tauri\Cargo.toml` 和 `src-tauri\tauri.conf.json` 中同步维护。
- 应用主要管理 Codex 账号、本地 `auth.json`、Codex 配置、Codex 启动入口、本机用量统计和自动更新。

## 项目结构

```text
codexx
├── src
│   ├── components
│   ├── router
│   ├── services
│   ├── stores
│   ├── styles
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
│       ├── codex_token_usage.rs
│       ├── codex_usage
│       ├── commands
│       ├── local_sync
│       ├── scheduler
│       ├── security
│       ├── status_sync
│       ├── storage
│       └── usage
├── CHANGELOG.md
├── README.md
├── package.json
├── tsconfig.json
└── vite.config.ts
```

## 前端边界

- 页面路由位于 `src\router\index.ts`，当前包含账号列表、账号详情、用量统计、Codex 配置和设置。
- Tauri 调用统一封装在 `src\services\index.ts`，新增后端命令时必须同步补齐服务方法和类型。
- 浏览器开发模式不得执行本地能力；服务层应返回空数据或明确错误。
- 账号状态集中在 `src\stores\account.ts`，加载账号时会先同步本地默认账号标记。
- 用量状态集中在 `src\stores\usage.ts`，按 `账号 ID + 周期` 缓存摘要和图表数据，多账号聚合必须累计缓存命中字段。
- 设置状态集中在 `src\stores\settings.ts`，保存失败时需要回滚前端状态。
- 修改界面时优先沿用 `src\styles\design-system.css` 和现有组件风格，不引入割裂的视觉体系。

## 后端边界

- Tauri 启动入口位于 `src-tauri\src\lib.rs`，新增命令必须同步注册到 `tauri::generate_handler!`。
- 命令模块位于 `src-tauri\src\commands`，命令层只做参数接收、状态协调和错误转换，不承载复杂业务规则。
- 账号模型和仓储位于 `src-tauri\src\account`，支持 `api_key`、`oauth_token`、`cookie_session` 和 `cli_profile`。
- OAuth 授权、刷新和校验位于 `src-tauri\src\auth`，错误信息不得泄露完整令牌。
- 本地 `auth.json` 同步位于 `src-tauri\src\local_sync`，负责读取、导入、稳定 ID、账号合并和默认账号写回。
- Codex CLI 和 Codex App 启动位于 `src-tauri\src\codex_runtime`，Windows 下需要兼容 `exe`、`cmd`、`bat` 和 `ps1`。
- Codex 配置读写位于 `src-tauri\src\codex_config.rs`，单字段保存应尽量只替换目标赋值行，保留其它字段、顺序和注释。
- 用量聚合位于 `src-tauri\src\usage`，Codex 会话日志导入位于 `src-tauri\src\codex_session_import.rs`。
- 后台状态检测和 Token 保活位于 `src-tauri\src\scheduler` 和 `src-tauri\src\status_sync`。

## 数据与安全边界

- SQLite 数据库默认位于用户目录 `.codex\CodexX\codexX.db`。
- 应用启动时会把旧应用数据目录中的 `codexX.db` 复制迁移到 `.codex\CodexX\codexX.db`，避免历史数据丢失。
- 数据库启用 WAL 和外键，核心表包括 `accounts`、`credentials`、`usage_records`、`codex_launch_sessions`、`api_usage_events` 和 `settings`。
- 凭证使用 AES-256-GCM 加密后写入数据库。
- 主密钥优先来自 `CODEX_MANAGER_MASTER_KEY`，未设置时使用系统凭据库；Windows 使用 Credential Manager，macOS 使用 Keychain。
- 系统凭据库后端必须具备真实持久化能力；不要绕过 `security` 模块直接读写主密钥。
- 应用不会自动读取 `.env` 文件；开发或自动化需要固定主密钥时，由当前 PowerShell 会话或启动脚本注入环境变量。
- `CODEX_MANAGER_MASTER_KEY` 支持 32 字节原文、64 位十六进制字符串和 base64 编码的 32 字节值。
- 导出的 `auth.json` 和 zip 包包含明文凭证，必须按敏感文件处理，不能写入日志、截图、文档示例或聊天记录。
- `CODEX_HOME\auth.json` 或用户目录 `.codex\auth.json` 是本地 Codex 默认认证文件，写回前必须生成 `auth.json.bak`。
- Cookie Session 和 CLI Profile 不能写回标准 Codex `auth.json`。

## 功能边界

### 账号与凭证

- 支持新增、更新、删除、默认账号、状态检测、本地同步、OAuth 登录、导入和导出。
- 本地同步会按 API Key 或 ChatGPT 账号身份生成稳定账号 ID，并可合并旧路径型本地同步账号。
- 切换账号会把可写回的凭证写入默认 `auth.json`，并刷新默认账号状态。
- 账号可展示 5 小时和 7 天 Codex 用量窗口、套餐类型、状态和错误信息。

### Codex 启动

- 支持受控 `codex exec --json`，交互式 Codex CLI，Codex CLI 终端和 Codex App。
- Codex CLI 发现顺序为 `CODEX_EXECUTABLE`、`codex.exe`、`codex.cmd`、`codex.bat`、`codex.ps1` 和 `codex`。
- 受控执行会解析 stdout 中的 JSONL usage 和 token_count 事件，并写入 `api_usage_events`。
- Codex App 启停当前只支持 Windows。

### 用量统计

- 用量数据来源包括旧聚合表 `usage_records` 和细粒度表 `api_usage_events`。
- 统计字段包含输入 Token、缓存命中 Token、输出 Token、请求次数和估算费用。
- 前端会传入本地时区偏移，后端按本地日期聚合今日、本周、本月、近一年、今年和本月等周期。
- `.codex\sessions` 导入只扫描与应用记录的 Codex 启动会话相关的候选 JSONL 日志。
- 导入事件使用稳定 ID 去重，避免重复刷新导致统计膨胀。

### Codex 配置

- 配置页读取用户级 `.codex\config.toml`。
- 整文件保存和单字段保存都必须先通过 TOML 校验。
- 写入使用临时文件和 `.bak` 备份，重命名失败时应尝试恢复。
- 动态字段需要用户填写完整字段名，例如 `mcp_servers.local.command`。

### 设置、托盘与更新

- 设置包括主题、自启、关闭窗口行为、后台检测间隔、Token 保活、自动更新和危险操作。
- 关闭窗口行为支持最小化到托盘和直接退出。
- 系统托盘提供打开主窗口、切换可用账号、重启 Codex App 和退出。
- 自动更新依赖 `src-tauri\tauri.conf.json` 中真实 updater endpoint、公钥和 GitHub Releases 产物。
- 如果仓库是私有仓库，匿名访问 `latest.json` 可能返回 `404`，Tauri updater 无法完成真实终端更新。

## 后端命令边界

前端通过 `src\services\index.ts` 调用 Tauri command。命令注册以 `src-tauri\src\lib.rs` 为准。

| 模块 | 命令 |
| --- | --- |
| 账号 | `create_account`、`update_account`、`delete_account`、`list_accounts`、`get_account`、`get_account_credential`、`switch_account`、`set_default_account` |
| 导入导出 | `export_account_auth_file`、`export_accounts`、`import_accounts` |
| 本地同步 | `sync_local_auth_file`、`sync_local_default_account` |
| 认证 | `refresh_token`、`validate_token`、`get_auth_status`、`prepare_oauth_login`、`open_oauth_login_url`、`complete_oauth_callback_login`、`cancel_oauth_login` |
| 状态 | `check_status`、`check_all_status` |
| 用量 | `fetch_usage`、`get_usage_stats`、`get_usage_chart_data`、`clear_usage_data` |
| Codex 启动 | `run_codex_exec_session`、`trigger_codex_short_conversation`、`open_codex_interactive_session`、`launch_codex_cli`、`get_codex_launcher_config`、`launch_codex_app`、`close_codex_app` |
| Codex 配置 | `read_codex_config_file`、`save_codex_config_field`、`save_codex_config_file` |
| 设置 | `get_settings`、`save_settings`、`set_autostart` |

## Tauri 能力边界

- 默认能力配置位于 `src-tauri\capabilities\default.json`。
- 当前开放窗口控制、外部链接、文件对话框、文件系统、通知、自启、更新器、进程重启和进程退出能力。
- 新增插件或能力时，必须说明为什么需要该权限，以及失败或滥用时的影响。
- 不要为了绕过权限问题扩大能力范围；优先缩小调用面。

## 开发命令

安装依赖：

```powershell
pnpm install
```

启动桌面开发环境：

```powershell
npm run dev
```

启动前端开发服务：

```powershell
npm run dev:web
```

前端构建：

```powershell
npm run build
```

后端编译：

```powershell
cargo check --manifest-path src-tauri\Cargo.toml --no-default-features
```

空白检查：

```powershell
git diff --check
```

## 针对性验证

Codex 配置页单字段保存：

```powershell
cargo test --manifest-path src-tauri\Cargo.toml codex_config --no-default-features
```

OAuth Token 保活：

```powershell
cargo test --manifest-path src-tauri\Cargo.toml token_refresh_tests --no-default-features
```

Codex 会话用量导入：

```powershell
cargo test --manifest-path src-tauri\Cargo.toml codex_session_import --no-default-features
```

用量按本地时区聚合：

```powershell
cargo test --manifest-path src-tauri\Cargo.toml chart_data_groups_api_events_by_query_timezone_date --no-default-features
```

编码和 BOM 检查示例：

```powershell
$bytes = [System.IO.File]::ReadAllBytes("AGENTS.md")
if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) { throw "AGENTS.md 包含 BOM" }
```

## 验证策略

- 不要频繁执行全量测试，优先选择与改动直接相关的最小可信验证。
- 涉及 Rust 后端逻辑时，至少执行对应的 `cargo test` 或 `cargo check`。
- 涉及前端类型、服务、store 或页面时，至少执行 `npm run build`。
- 涉及文档、配置或格式时，至少执行 `git diff --check`，并人工阅读关键差异。
- 如果无法执行自动化测试，必须给出明确、可复现的人工验证步骤，包括前置条件、操作步骤和预期结果。

## 发布与更新

- 发布版本时同步更新 `package.json`、`src-tauri\Cargo.toml`、`src-tauri\Cargo.lock` 和 `src-tauri\tauri.conf.json`。
- GitHub Releases 稳定更新入口为 `https://github.com/weimin96/CodexX/releases/latest/download/latest.json`。
- `src-tauri\tauri.conf.json` 中的 updater `pubkey` 必须与 GitHub Actions 使用的签名私钥匹配。
- `TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 只能作为仓库 Secret 或本地安全输入，不得提交。
- 本地生成签名密钥时使用 PowerShell，并确保私钥不进入仓库：

```powershell
pnpm exec tauri signer generate -- --ci -w "$env:USERPROFILE\.tauri\codexx-updater.key"
```

## 交付说明要求

最终回复必须包含变更内容和验证方式。

变更内容需要说明：

- 修改了哪些文件。
- 每处修改的原因。
- 修改前后的行为差异。

验证方式需要说明：

- 执行过的自动化测试命令和结果。
- 或者明确可复现的人工验证步骤，包括前置条件、操作步骤和预期结果。

如果验证没有执行，必须说明原因和剩余风险。
