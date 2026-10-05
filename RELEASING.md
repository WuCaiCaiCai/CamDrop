# 发布新版本

本项目用 **GitHub Actions 一键发布**：触发 `release.yml`，它会自动递增版本号、
提交并打 tag、在四个平台编译、打包成多种安装包，并创建 GitHub Release。

你（或 AI）只需要触发它，然后等结果。

## 触发发布

### 方式一：对 AI 说一句话（推荐）

> 更新版本并发版（patch / minor / major）

AI 会执行：

```bash
gh workflow run release.yml -f bump=patch     # 小版本用 minor，大版本用 major
gh run watch                                   # 跟进进度，完成后给出 Release 链接
```

### 方式二：网页手动触发

仓库 **Actions → Release → Run workflow**，选择 `patch` / `minor` / `major`，点 **Run workflow**。

### 方式三：命令行（你本机）

```bash
gh workflow run release.yml -f bump=patch
```

> `bump` 不填默认 `patch`。版本号规则遵循语义化版本 `vX.Y.Z`：
> `patch` 修 bug（0.1.0 → 0.1.1），`minor` 加功能（0.1.1 → 0.2.0），`major` 破坏性变更（0.2.0 → 1.0.0）。

## 工作流做了什么

`.github/workflows/release.yml`，三个 job：

1. **bump**：读取 `Cargo.toml` 当前版本 → 按 `bump` 递增 → `cargo set-version` 更新
   `Cargo.toml`/`Cargo.lock` → 提交 `chore(release): vX.Y.Z` → 推送 → 打并推送 tag `vX.Y.Z`。
2. **build**（矩阵，四个平台）：
   | 平台 | 产物 |
   |---|---|
   | Windows x86_64 | `camdrop-<tag>-windows-x86_64.zip` |
   | Linux x86_64 | `camdrop_<ver>_amd64.deb`、`camdrop-<ver>-1.x86_64.rpm`、`CamDrop-<ver>-x86_64.AppImage`、`camdrop-<tag>-linux-x86_64.tar.gz` |
   | macOS x86_64 | `camdrop-<tag>-macos-x86_64.tar.gz` |
   | macOS aarch64 | `camdrop-<tag>-macos-aarch64.tar.gz` |
3. **release**：汇总所有产物 → 创建 GitHub Release（自动生成 release notes）并挂上全部文件。

zip / tar.gz 里包含：可执行文件、`README.md`、`LICENSE`、`assets/icon.png`。

## 一次性前置条件

- 仓库 **Settings → Actions → General → Workflow permissions** 设为 **Read and write**
  （打 tag、创建 Release 需要）。
- `main` 分支不要开启“禁止直接 push”的分支保护（工作流要推送版本提交）。
- 用 `gh` 触发时，本机需先 `gh auth login`。

## 产物说明与已知限制

- **架构**：Linux 只出 **x86_64** 的 deb/rpm/AppImage；macOS 同时出 Intel 与 Apple Silicon。
- **AppImage / 运行依赖**：图形界面走 OpenGL，**不打包显卡驱动**（由系统提供）；
  依赖 X11/Wayland、`libGL`。deb/rpm 已声明相关依赖。
- **中文字体**：界面中文依赖**系统字体**（不内嵌）。缺字体时会显示方框。
- **签名**：deb/rpm 未做 GPG 签名，AppImage/Windows/macOS 未签名，首次运行可能有
  系统安全提示（Windows SmartScreen、macOS Gatekeeper）。
- **首次发布前**：建议先在本地跑一遍 `cargo fmt`、`cargo clippy --all-targets -- -D warnings`、
  `cargo test`、`cargo build --release`，确保绿色。

## 失败排查

- 打开 **Actions → 对应 run**，看哪个 job/step 红了。
- 常见点：
  - Linux 编译缺库 → 检查 `install Linux build dependencies`。
  - `cargo deb` / `cargo generate-rpm` 找不到二进制 → 确认 Linux 用 native 构建（`target/release/camdrop`），
    且 `Cargo.toml` 里的 `assets` 路径一致。
  - AppImage 步骤 → `appimagetool` 用 `--appimage-extract-and-run` 已规避 CI 无 FUSE 的问题。
  - Release 创建失败 → 检查 workflow permissions 是否为 Read and write。

## 手工兜底（不用 Actions 时）

1. 本地改版本：`cargo set-version X.Y.Z`（需 `cargo install cargo-edit`），提交。
2. 打 tag 并推送：`git tag vX.Y.Z && git push origin vX.Y.Z`。
3. 本地编译打包（Linux）：`cargo build --release`、`cargo deb`、`cargo generate-rpm`、
   参照 `release.yml` 里的 AppImage 步骤。
4. 在 GitHub 手动创建 Release 并上传产物。
