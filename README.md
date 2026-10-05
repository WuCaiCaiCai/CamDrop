# CamDrop

把相机存储卡里的照片和视频按拍摄日期归好档。

插上卡，程序自己找到卡，读每张照片的拍摄时间，分到 `年份/月_日` 目录里，再移动到指定位置。调色软件（Darktable、Lightroom）生成的 `.xmp` 侧边栏文件会跟着照片一起走。用 Rust 写，编译出来是一个可执行文件，不需要 Python，也不需要额外装运行时。

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## 功能

- 自动识别存储卡。扫描已挂载的磁盘，靠 RAW 文件（`.nef` `.cr3` `.arw` `.dng` `.raf` 等）判断哪块是相机卡。Windows 跳过系统盘；Linux 只在 `/media`、`/mnt`、`/run/media` 下找；macOS 在 `/Volumes` 下找。
- 按拍摄日期归档。优先读 EXIF 的 `DateTimeOriginal`，读不到就退回文件修改时间，目录形如 `2026/09_24`。
- 同步 `.xmp`。照片移动时把同名的 `.xmp` 一起搬过去；搬完之后如果卡上还有没跟上的 xmp，会再扫一遍，按文件名匹配补迁。
- 跨盘移动。从 F: 移到 E: 时 `rename` 会报 CrossesDevices，程序自动改成「复制 + 删除」。
- 只读文件处理。相机写的文件可能带只读属性，删除失败时先去掉只读再重试。
- 重名不覆盖。目标目录已有同名文件时自动加 `_1`、`_2`。
- 试运行。只看归档计划，不动任何文件。

## 环境要求

运行支持 Windows、Linux 和 macOS。界面渲染走 OpenGL。

从源码编译需要 Rust 1.94 或更新的 stable 工具链。

界面文字是中文，需要系统里有中文字体。程序在启动时从系统字体目录里找一个来用（Windows 用微软雅黑，Linux 用 Noto CJK 或文泉驿，macOS 用苹方），字体不打包进程序。

## 编译

```bash
git clone https://github.com/WuCaiCaiCai/CamDrop.git
cd CamDrop
cargo build --release
```

产物在 `target/release/` 下，Linux/macOS 是 `camdrop`，Windows 是 `camdrop.exe`。

也可以直接到 [Releases](../../releases) 页面下载预编译的版本。

## 使用

运行程序后：

1. 程序扫描挂载的磁盘，把识别到的相机卡列出来。
2. 勾选要归档的卡；如果没识别到，点「添加文件夹」手动选。
3. 选目标目录，默认是程序所在目录下的 `RAW`。
4. 点「开始归档」。

界面上的两个开关：

- 仅复制：保留源卡里的文件，只往目标目录复制一份。
- 试运行：只打印每个文件会去哪，不移动、不复制。

移动过程在后台进行，界面下方的日志区会逐条显示结果，出错的文件会标出来。

## 归档结构

```text
RAW/
└── 2026/
    ├── 09_24/
    │   ├── _DSC1024.NEF
    │   ├── _DSC1024.NEF.xmp
    │   └── _DSC1025.JPG
    └── 10_05/
        └── DSC_2048.NEF
```

## 项目结构

```text
src/
├── main.rs        程序入口
├── app.rs         界面状态、后台归档线程、日志
├── lib.rs         对外导出模块和默认扩展名
├── detector.rs    挂载点枚举、RAW 特征探测
├── metadata.rs    EXIF 拍摄时间、mtime 回退、目录名
├── organizer.rs   文件收集、去重命名、跨盘移动、xmp 补迁
└── xmp.rs         .xmp 路径处理
tests/
└── integration_test.rs
```

## 开发

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

release 配置开了体积优化（`opt-level = z`、LTO、strip），Windows 下可执行文件约 5.6 MB。

## 许可

[MIT](LICENSE)
