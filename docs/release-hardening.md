# 发布可信度说明

## 已落地

| 项目 | 作用 | 位置 |
| --- | --- | --- |
| Tauri updater 签名 | 安装更新前校验更新包签名 | `src-tauri/tauri.conf.json` |
| GitHub Releases updater 入口 | 提供 `latest.json` 和安装包分发 | `.github/workflows/release-on-tag.yml` |
| SHA256SUMS | 供用户手工校验 Release 资产完整性 | `.github/workflows/release-on-tag.yml` |
| PR 与 main CI | 在发布前执行前端构建、Rust 格式、Clippy、测试和依赖审查 | `.github/workflows/ci.yml` |
| Dependabot | 自动发现 npm、Cargo 与 GitHub Actions 依赖更新 | `.github/dependabot.yml` |
| audit 门禁 | 检查 npm 生产依赖和 Cargo 已知漏洞 | `.github/workflows/ci.yml` |

## 待接入

| 项目 | 接入条件 | 风险 |
| --- | --- | --- |
| macOS notarization | Apple Developer 账号、签名证书和 notarization 凭据 | 未公证包可能被 Gatekeeper 拦截 |
| Windows Authenticode | 代码签名证书和签名密钥保管策略 | 未签名包更容易触发 SmartScreen 提示 |
| SBOM | 选定 CycloneDX 或 SPDX 生成工具，并固定版本与上传格式 | 缺少依赖清单会降低供应链追溯能力 |

## 发布前检查

- 确认 `TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 只存在于仓库 Secret 或本机安全输入中。
- 确认版本号在 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 和 `src-tauri/tauri.conf.json` 中一致。
- 确认 `pnpm build`、`cargo check --manifest-path src-tauri\Cargo.toml --no-default-features` 与 `git diff --check` 通过。
- tag 发布完成后，确认 Release 中存在安装包、签名文件、`latest.json` 和 `SHA256SUMS`。
