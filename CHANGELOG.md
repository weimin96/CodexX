# 变更记录

## 0.1.1 - 2026-04-20

### 新增

- 接入基于 GitHub Releases `latest.json` 的稳定版在线更新发布链路。
- 新增 updater 签名密钥生成与 GitHub Actions Secret 约束说明。

### 修改

- `src-tauri\tauri.conf.json` 改为启用 `createUpdaterArtifacts`，并固定使用 GitHub Releases 稳定更新地址。
- `release-on-tag.yml` 升级为使用 `tauri-apps/tauri-action@v1` 产出 updater 资产与 `latest.json`。
- 设置页版本展示改为读取工程版本号，避免手工发版后界面版本漂移。
- Rust 请求 `User-Agent` 改为读取编译期版本，避免继续发送旧版本号。
