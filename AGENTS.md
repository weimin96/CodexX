# AGENTS

## 工作环境

- 默认在 Windows PowerShell 中执行命令。
- 文档、注释和提交说明使用中文。
- 文件编码使用 UTF-8，不使用 BOM。
- 不使用 emoji。

## 开发规则

- 先阅读相关代码，再修改实现。
- 复杂任务先更新 `task_plan.md`、`progress.md` 或 `findings.md`，再实施。
- 一个独立问题对应一个独立 commit。
- 不回滚用户已有改动。
- 不把 Token、API Key、refresh token、完整 `auth.json` 或主密钥写入日志、文档示例和前端状态。
- 修改凭证、账号切换、导入导出、更新器、自动任务和后台调度时，需要明确失败路径。

## 验证命令

后端编译：

```powershell
cargo check --manifest-path src-tauri\Cargo.toml --no-default-features
```

前端构建：

```powershell
npm run build
```

空白检查：

```powershell
git diff --check
```

针对性单测：

```powershell
cargo test --manifest-path src-tauri\Cargo.toml codex_config --no-default-features
cargo test --manifest-path src-tauri\Cargo.toml token_refresh_tests --no-default-features
cargo test --manifest-path src-tauri\Cargo.toml codex_session_import --no-default-features
```

## 项目边界

- 数据库位于用户目录 `.codex\CodexX\codexX.db`。
- 正式运行默认使用系统凭据库保存主密钥；`CODEX_MANAGER_MASTER_KEY` 只作为显式覆盖入口。
- 账号导出的 `auth.json` 和 zip 包包含明文凭证，必须按敏感文件处理。
- Codex 配置页的单字段保存应尽量只替换目标字段，不重新序列化整份 TOML。
- 自动更新依赖 `src-tauri\tauri.conf.json` 中真实 updater endpoint 和公钥；占位配置不能验证真实在线更新。
