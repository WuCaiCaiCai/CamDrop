# 发布新版本

发布由 **推送 tag 触发** GitHub Actions：本地把版本号改好、提交、打 `vX.Y.Z` 标签并 push，
Actions 就会在 Windows / Linux / macOS 编译、打包并创建 GitHub Release。全程使用你已保存的 git 凭据，
不需要 GitHub token，也不会每次让你重新登录。

## 推荐方式：本地一行命令

```bash
pwsh scripts/release.ps1 patch     # 或 minor / major
```

脚本会自动：

1. 读取 `Cargo.toml` 当前版本，按 `patch`/`minor`/`major` 递增；
2. 改写 `Cargo.toml` 的版本号，`cargo build --release` 顺带刷新 `Cargo.lock`；
3. `git commit -m "chore(release): vX.Y.Z"`；
4. `git tag vX.Y.Z` → `git push` → `git push origin vX.Y.Z`；
5. 打印 Actions 页面地址。

推上去的 tag 会触发 `.github/workflows/release.yml`。等它跑完，Release 页就有安装包了。

> 版本规则（语义化）：`patch` 0.1.0 → 0.1.1；`minor` 0.1.1 → 0.2.0；`major` 0.2.0 → 1.0.0。

### 手动等价操作（不用脚本时）

```bash
# 1. 改 Cargo.toml 里的 version = "X.Y.Z"
# 2. 刷新锁文件
cargo build --release
# 3. 提交并打 tag 推送
git commit -am "chore(release): vX.Y.Z"
git tag vX.Y.Z
git push && git push origin vX.Y.Z
```

## 工作流做了什么

`.github/workflows/release.yml` 监听 `push` 的 `v*` 标签：

1. **meta**：从 tag 解析出 `tag` / `version`。
2. **build**（矩阵，三平台）：
   | 平台 | 产物 |
   |---|---|
   | Windows x86_64 | `camdrop-<tag>-windows-x86_64.zip` |
   | Linux x86_64 | `camdrop_<ver>_amd64.deb`、`camdrop-<ver>-1.x86_64.rpm`、`CamDrop-<ver>-x86_64.AppImage`、`camdrop-<tag>-linux-x86_64.tar.gz` |
   | macOS arm64 | `camdrop-<tag>-macos-aarch64.tar.gz` |
3. **release**：汇总所有产物，创建 GitHub Release（自动生成 release notes）。

zip / tar.gz 里包含：可执行文件、`README.md`、`LICENSE`、`assets/icon.png`。

## 备用方式：手动触发（需要 GitHub API 权限）

`.github/workflows/release.yml` 也支持 `workflow_dispatch`（会自动递增版本、提交、打 tag）：

```bash
gh workflow run release.yml -f bump=patch    # 需要 gh 且已登录
```

或在仓库 **Actions → Release → Run workflow** 手动点。

> 说明：手动方式需要 GitHub 认证；推荐用上面的 **tag 推送** 方式，只用 git 凭据。

## 一次性前置条件

- 仓库 **Settings → Actions → General → Workflow permissions** 设为 **Read and write**
  （创建 Release 需要；tag 推送本身不需要额外权限）。
- 本地 `git push` 正常（凭据管理器已保存）。

## 产物说明与已知限制

- **架构**：Linux 只出 **x86_64** 的 deb/rpm/AppImage；macOS 只出 Apple Silicon（arm64）。
- **AppImage / 运行依赖**：图形界面走 OpenGL，**不打包显卡驱动**（由系统提供）；
  依赖 X11/Wayland、`libGL`。deb/rpm 已声明相关依赖。
- **中文字体**：界面中文依赖**系统字体**（不内嵌），缺字体会显示方框。
- **签名**：deb/rpm 未做 GPG 签名，AppImage/Windows/macOS 未签名，首次运行可能有系统安全提示。

## 失败排查

- 打开 **Actions → 对应 run**，看哪个 job/step 红了。
- 常见点：
  - Linux 编译缺库 → 检查 `Install Linux build dependencies`。
  - `cargo deb` / `cargo generate-rpm` 找不到二进制 → 确认 Linux 用 native 构建（`target/release/camdrop`）。
  - AppImage 步骤 → 已用 `--appimage-extract-and-run` 规避 CI 无 FUSE。
  - Release 创建失败 → 检查 workflow permissions 是否为 Read and write。

## 待办

剩余工作（发布流程验证、产物核对、界面确认等）见 [TODO.md](TODO.md)。
