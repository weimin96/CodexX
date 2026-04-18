# Codex 账号管理工具

一个功能完整的 Windows 桌面应用，支持多账号管理、用量统计、状态检测，基于 **Tauri 2 + Vue 3 + Rust** 构建。

---

## 目录

- [功能特性](#功能特性)
- [技术架构](#技术架构)
- [项目结构](#项目结构)
- [快速开始](#快速开始)
- [核心模块](#核心模块)
- [安全设计](#安全设计)
- [数据库设计](#数据库设计)
- [API 参考](#api-参考)
- [构建发布](#构建发布)

---

## 功能特性

### 账号管理
- ✅ 新增、编辑、删除、切换账号
- ✅ 支持 API Key / OAuth Token / Cookie Session / CLI Profile 四种认证方式
- ✅ 设置默认账号
- ✅ 加密导入 / 导出（AES-256-GCM + PBKDF2）
- ✅ 账号颜色标识、头像文字

### 状态检测
- ✅ 单账号 / 全部检测
- ✅ 状态分类：正常 / 警告 / 异常 / 过期 / 未知
- ✅ 后台定时自动检测（可配置间隔）
- ✅ 实时事件推送至前端

### 用量统计
- ✅ 日 / 周 / 月三档时间周期
- ✅ 输入 Token、输出 Token、请求次数、费用估算
- ✅ ECharts 折线图 + 柱状图
- ✅ 明细数据表格（可排序、分页）

### 系统能力
- ✅ 自定义无边框标题栏
- ✅ 系统托盘（关闭最小化至托盘）
- ✅ 开机自启（可选）
- ✅ Tauri updater 自动更新
- ✅ 深色主题

### 安全
- ✅ AES-256-GCM 加密所有凭证
- ✅ 主密钥存储于系统凭据库（Windows Credential Store）
- ✅ 导出文件 PBKDF2 密码派生 + AES-256-GCM 加密
- ✅ 禁止明文存储任何密钥

---

## 技术架构

```
┌─────────────────────────────────────────────────────────┐
│  UI 层  Vue 3 + TypeScript + Naive UI + ECharts          │
├─────────────────────────────────────────────────────────┤
│  状态层  Pinia (accountStore / usageStore / settingsStore)│
├─────────────────────────────────────────────────────────┤
│  调用层  Tauri invoke + event（services/index.ts）        │
├─────────────────────────────────────────────────────────┤
│  业务层  Rust（account / auth / usage / security / ...）  │
├─────────────────────────────────────────────────────────┤
│  存储层  SQLite（rusqlite）+ 系统 Keyring                  │
└─────────────────────────────────────────────────────────┘
```

### 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| 桌面框架 | Tauri 2 | 原生窗口、托盘、更新、权限 |
| 前端框架 | Vue 3 + Composition API | 响应式 UI |
| 类型系统 | TypeScript（strict） | 端到端类型安全 |
| 构建工具 | Vite 5 | 极速热更新 |
| 状态管理 | Pinia | 模块化 store |
| UI 组件 | Naive UI | 暗色主题友好 |
| 图表 | ECharts 5 | 折线 / 柱状图 |
| 路由 | Vue Router 4 | SPA 导航 |
| 后端语言 | Rust + tokio | 异步业务逻辑 |
| 数据库 | SQLite（rusqlite bundled） | 本地持久化 |
| 加密 | AES-256-GCM（aes-gcm crate） | 凭证加密 |
| 密钥管理 | keyring crate | 系统凭据库集成 |
| HTTP 客户端 | reqwest（rustls） | 状态检测 |

---

## 项目结构

```
codex-manager/
├── src/                          # 前端源码
│   ├── main.ts                   # 应用入口
│   ├── App.vue                   # 根组件（主题配置）
│   ├── router/
│   │   └── index.ts              # Vue Router 路由配置
│   ├── stores/
│   │   ├── account.ts            # 账号状态管理
│   │   ├── usage.ts              # 用量状态管理
│   │   └── settings.ts           # 设置状态管理
│   ├── services/
│   │   └── index.ts              # Tauri invoke 封装层
│   ├── types/
│   │   └── index.ts              # TypeScript 类型定义
│   ├── views/
│   │   ├── AccountListView.vue   # 账号列表页
│   │   ├── AccountDetailView.vue # 账号详情页
│   │   ├── UsageView.vue         # 用量统计页
│   │   └── SettingsView.vue      # 设置页
│   └── components/
│       ├── common/
│       │   ├── AppLayout.vue     # 主布局（标题栏 + 侧边栏）
│       │   └── StatusDot.vue     # 状态指示点
│       └── account/
│           ├── AccountCard.vue   # 账号卡片
│           └── CreateAccountModal.vue  # 新建账号弹窗
│
├── src-tauri/                    # Rust 后端
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json          # Tauri 2 权限配置
│   └── src/
│       ├── main.rs               # 二进制入口
│       ├── lib.rs                # 应用初始化、插件注册、托盘
│       ├── error.rs              # 统一错误类型
│       ├── account/mod.rs        # 账号模型 + Repository
│       ├── auth/mod.rs           # 认证服务（Token 验证）
│       ├── usage/mod.rs          # 用量 Repository
│       ├── security/mod.rs       # AES-256-GCM 加密工具
│       ├── storage/mod.rs        # SQLite 数据库初始化
│       ├── scheduler/mod.rs      # 后台定时检测
│       └── commands/
│           ├── mod.rs            # 命令模块聚合
│           ├── account.rs        # 账号 Tauri commands
│           ├── auth.rs           # 认证 Tauri commands
│           ├── status.rs         # 状态检测 commands
│           ├── usage.rs          # 用量 commands
│           └── settings.rs       # 设置 commands
│
├── index.html
├── package.json
├── vite.config.ts
├── tsconfig.json
└── README.md
```

---

## 快速开始

### 前置要求

| 工具 | 版本 | 说明 |
|------|------|------|
| Node.js | ≥ 18 | 前端构建 |
| pnpm | ≥ 8 | 包管理器 |
| Rust | ≥ 1.77 | 后端编译 |
| Visual Studio Build Tools | 2019+ | Windows C++ 工具链 |
| WebView2 | 内置于 Win10/11 | Tauri 运行时 |

### 安装 & 运行

```bash
# 1. 克隆项目
git clone https://github.com/your-org/codex-manager.git
cd codex-manager

# 2. 安装前端依赖
pnpm install

# 3. 开发模式（热更新）
pnpm tauri dev

# 4. 生产构建
pnpm tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`，包含安装包（`.msi`）和便携版（`.exe`）。

---

## 核心模块

### Rust 命令接口（Tauri Commands）

所有 Rust 端暴露给前端的接口均通过 `tauri::command` 宏注册：

#### 账号管理

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `create_account` | `CreateAccountInput` | `Account` | 创建账号并加密存储凭证 |
| `update_account` | `UpdateAccountInput` | `Account` | 更新账号信息 |
| `delete_account` | `id: String` | `void` | 删除账号及其凭证 |
| `list_accounts` | — | `Account[]` | 列出所有账号 |
| `get_account` | `id: String` | `Account` | 获取单个账号 |
| `switch_account` | `id: String` | `void` | 切换默认账号 |
| `set_default_account` | `id: String` | `void` | 设置默认账号 |
| `export_accounts` | `password: String` | `String` | 导出加密数据 |
| `import_accounts` | `encrypted_data, password` | `usize` | 导入账号（返回数量） |

#### 认证

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `validate_token` | `account_id: String` | `AuthCheckResult` | 验证 Token 有效性 |
| `refresh_token` | `account_id: String` | `AuthCheckResult` | 刷新并重新验证 |
| `get_auth_status` | `account_id: String` | `String` | 获取认证状态字符串 |

#### 状态检测

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `check_status` | `account_id: String` | `StatusCheckResult` | 检测单账号状态 |
| `check_all_status` | — | `StatusCheckResult[]` | 批量检测所有账号 |

#### 用量统计

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `fetch_usage` | `account_id: String` | `void` | 触发用量拉取 |
| `get_usage_stats` | `UsageQuery` | `UsageSummary` | 获取汇总统计 |
| `get_usage_chart_data` | `UsageQuery` | `ChartDataPoint[]` | 获取图表数据 |

#### 设置

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_settings` | — | `AppSettings` | 读取所有设置 |
| `save_settings` | `settings: Object` | `void` | 保存设置 |
| `set_autostart` | `enabled: bool` | `void` | 开关开机自启 |

### Tauri 事件

后端通过 `tauri::Emitter` 向前端推送以下事件：

| 事件名 | 载荷 | 触发时机 |
|--------|------|----------|
| `account-status-updated` | `{ account_id, status, message }` | 后台定时检测完成时 |

---

## 安全设计

### 凭证加密流程

```
用户输入明文 API Key
       ↓
security::encrypt()
       ↓
get_master_key()  ←→  系统 Keyring（Windows Credential Store）
       ↓
AES-256-GCM 加密（随机 Nonce）
       ↓
Base64 编码
       ↓
存入 SQLite credentials 表
```

### 导出加密流程

```
原始账号数据（JSON）
       ↓
security::encrypt_export(password)
       ↓
PBKDF2-HMAC-SHA256（100,000 轮）派生密钥
       ↓
AES-256-GCM 加密（随机 Salt + Nonce）
       ↓
Base64 编码 → 输出给用户
```

---

## 数据库设计

```sql
-- 账号表
CREATE TABLE accounts (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    auth_type   TEXT NOT NULL,     -- api_key | oauth_token | cookie_session | cli_profile
    email       TEXT,
    organization TEXT,
    is_default  INTEGER DEFAULT 0,
    is_active   INTEGER DEFAULT 1,
    status      TEXT DEFAULT 'unknown',
    status_message TEXT,
    color       TEXT DEFAULT '#18a058',
    avatar_text TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    last_checked_at TEXT
);

-- 凭证表（加密存储）
CREATE TABLE credentials (
    id              TEXT PRIMARY KEY,
    account_id      TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    credential_type TEXT NOT NULL,
    encrypted_value TEXT NOT NULL,  -- AES-256-GCM 密文
    expires_at      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- 用量记录表
CREATE TABLE usage_records (
    id              TEXT PRIMARY KEY,
    account_id      TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    date            TEXT NOT NULL,  -- YYYY-MM-DD
    input_tokens    INTEGER DEFAULT 0,
    output_tokens   INTEGER DEFAULT 0,
    request_count   INTEGER DEFAULT 0,
    estimated_cost  REAL DEFAULT 0.0,
    model           TEXT,
    created_at      TEXT NOT NULL
);

-- 设置表
CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
```

---

## 构建发布

### 配置自动更新

在 `tauri.conf.json` 中配置更新服务器地址：

```json
{
  "plugins": {
    "updater": {
      "endpoints": ["https://your-update-server.com/{{target}}/{{arch}}/{{current_version}}"],
      "pubkey": "YOUR_PUBLIC_KEY_HERE"
    }
  }
}
```

生成签名密钥对：
```bash
pnpm tauri signer generate -w ~/.tauri/codex-manager.key
```

### 构建命令

```bash
# Debug 构建
pnpm tauri build --debug

# Release 构建（含代码签名）
pnpm tauri build

# 仅构建前端
pnpm build
```

---

## 扩展指南

### 新增数据源（用量 API 扩展）

在 `src-tauri/src/usage/mod.rs` 中扩展 `UsageRepository`，新增来自 OpenAI Usage API 的真实数据拉取：

```rust
pub async fn fetch_from_api(&self, account_id: &str, api_key: &str) -> AppResult<()> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.openai.com/v1/usage")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;
    // 解析并写入 usage_records
    Ok(())
}
```

### 新增认证方式

在 `src-tauri/src/auth/mod.rs` 中扩展 `AuthService`，并在 `src-tauri/src/account/mod.rs` 的 `AuthType` 枚举中添加新变体。

---

## License

MIT
