# CodexX

<p align="center">
  <img src="docs/readme/codexx-overview.png" alt="CodexX 项目概览图" />
</p>

CodexX 是一个基于 Tauri 2、Vue 3 和 Rust 的桌面应用，用于管理 Codex 账号、本地认证文件、Codex 配置和本机用量统计。

## 功能概览

### 账号与凭证

- 账号管理
  - 支持 API Key、OAuth Token、本地配置导入导出账号。
  - 5小时剩余与7天剩余预览。显示5小时与7天剩余量与重置时间。
  - 账号切换。一键切换新账号。
  - 5小时预热、7天预热快。触发账号下次重置时开始计时。
  - Token 保活。定期刷新 OAuth Token，保持账号持续可用。
- 仪表盘
  - 显示所有账号的 Codex 额度用量统计。
- Codex 配置
  - 快速配置 Codex 相关配置。
- 快速启动
  - 一键重启 Codex App。

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