# 变更记录

## 0.1.3 - 2026-04-22

### 修复

- 修正 Token 用量统计的本地日期口径，避免已产生今日用量时仍显示为 0。
- 修正账号详情页邮箱标题和 Token 数值字号过大的展示问题。

### 修改

- 账号详情页“本月用量”改为“Token用量”，并按今日、本月、今年展示总 Token、输入 Token 和输出 Token。
- Token 数值按 K、M、B、T 自动换算，避免大数值展示不易阅读。
- 仪表盘“近一年账号明细”改为“账号用量明细”，增加今年、本月、今天筛选，默认展示今天。
- 收紧账号详情页头部布局，减少账号基础信息下方空白。
- 移除 Plus/Pro 到期时间展示逻辑，避免在没有可靠数据源时展示不可信信息。
- 发布工作流升级到 Node.js 22，并精确固定 pnpm 版本以保证依赖安装稳定。

## 0.1.2 - 2026-04-20

### 修复

- 修正 GitHub Actions 发布工作流中的 `tauri-apps/tauri-action` 引用，改为真实存在的稳定标签 `v0.6.2`。
- 补发稳定版补丁号，避免重写已经推送到远端的 `v0.1.1` 标签。

## 0.1.1 - 2026-04-20

### 新增

- 接入基于 GitHub Releases `latest.json` 的稳定版在线更新发布链路。
- 新增 updater 签名密钥生成与 GitHub Actions Secret 约束说明。

### 修改

- `src-tauri\tauri.conf.json` 改为启用 `createUpdaterArtifacts`，并固定使用 GitHub Releases 稳定更新地址。
- `release-on-tag.yml` 改为使用官方 `tauri-apps/tauri-action` 发布 updater 资产与 `latest.json`。
- 设置页版本展示改为读取工程版本号，避免手工发版后界面版本漂移。
- Rust 请求 `User-Agent` 改为读取编译期版本，避免继续发送旧版本号。
