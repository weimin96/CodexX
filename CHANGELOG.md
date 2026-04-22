# 变更记录

## 0.1.4 - 2026-04-22

### 修复

- 调整数据库存放位置。
- 增加关闭窗口配置项。
- 更新提示优化。
- token用量统计修正。


## 0.1.3 - 2026-04-22

### 修复

- 修正 Token 用量统计。

### 修改

- Token 数值自动换算。
- 仪表盘样式优化
- 优化账号详情页样式布局优化

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
