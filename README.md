# CamDrop

> 相机存储卡归档工具：自动找到存储卡，按拍摄日期整理照片和视频，并同步 Darktable / Lightroom 生成的 `.xmp` 侧边栏文件。

🌐 [English](README_EN.md) | 简体中文

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## 📖 简介

CamDrop 是一个用 Rust 写的桌面小工具。插上相机存储卡后，它会扫描已挂载的磁盘，靠 RAW 文件判断哪块是相机卡，再把卡里的照片、视频按拍摄时间分到 `年份/月_日` 目录，移动到目标位置。移动过程中，和照片同名的 `.xmp` 文件会一起搬走。

界面基于 eframe / egui，编译后是一个可执行文件，不需要 Python 或任何额外运行时。

## ✨ 功能特性

### 🎯 核心功能

| 功能 | 说明 |
|------|------|
| 存储卡自动识别 | 扫描已挂载磁盘，依据 `.nef` `.cr3` `.arw` `.dng` 等 RAW 文件判断相机卡；Windows 跳过系统盘，Linux 只在 `/media`、`/mnt`、`/run/media` 下查找，macOS 在 `/Volumes` 下查找 |
| 按拍摄日期归档 | 读取 EXIF 的 `DateTimeOriginal` 建立目录；读不到时退回文件修改时间 |
| XMP 同步 | 照片移动时把同名的 `.xmp` 一并搬走；搬完后若卡上仍有遗漏的 xmp，会再次扫描并按文件名匹配补迁 |
| 跨盘安全移动 | `rename` 遇到 CrossesDevices 时自动降级为「复制 + 删除」 |
| 只读文件处理 | 删除失败时先去掉只读属性再重试 |
| 重名保护 | 目标已有同名文件时自动追加 `_1`、`_2`，不覆盖 |

### 🖥️ 界面与操作

| 特性 | 说明 |
|------|------|
| 源卡勾选 | 识别到的存储卡以列表展示，可多选；未识别到时手动「添加文件夹」 |
| 目标目录 | 默认是程序所在目录下的 `RAW`，可用「浏览」修改 |
| 仅复制 | 保留源卡文件，只向目标目录复制一份 |
| 试运行 | 只打印归档计划，不修改任何文件 |
| 进度与日志 | 后台线程执行，界面显示进度条，逐条记录每个文件的处理结果，失败项单独标出 |

### 🧩 支持格式

| 类型 | 扩展名 |
|------|--------|
| 照片 | `.jpg` `.jpeg` `.nef` `.cr3` `.arw` `.dng` |
| 视频 | `.mp4` `.mov` |
| 探测用 RAW 签名 | 归档格式之外，还包括 `.raf` `.orf` `.rw2` `.pef` `.srw` `.nrw` |

## 🚀 快速开始

### 环境要求

| 项目 | 要求 |
|------|------|
| 操作系统 | Windows / Linux / macOS |
| 图形接口 | OpenGL（由显卡驱动提供） |
| 中文字体 | 系统需装有中文字体：Windows 微软雅黑、Linux Noto CJK 或文泉驿、macOS 苹方（程序读取系统字体，不打包） |
| 编译工具链 | Rust 1.94 或更新的 stable |

### 方式一：下载预编译版本（推荐）

1. 前往 [Releases](../../releases) 页面
2. 下载对应系统的可执行文件
3. 双击运行

### 方式二：源码编译

```bash
git clone https://github.com/WuCaiCaiCai/CamDrop.git
cd CamDrop
cargo build --release
```

产物位于 `target/release/`：Linux / macOS 为 `camdrop`，Windows 为 `camdrop.exe`。

## 📖 使用指南

### 操作步骤

| 步骤 | 操作 |
|------|------|
| 1 | 运行程序，等待扫描完成，界面列出识别到的相机卡 |
| 2 | 勾选需要归档的卡；没有识别到时点「添加文件夹」手动选择 |
| 3 | 确认目标目录（默认为程序目录下的 `RAW`） |
| 4 | 按需勾选「仅复制」或「试运行」 |
| 5 | 点击「开始归档」，在下方日志区查看结果 |

### 选项说明

| 选项 | 默认 | 说明 |
|------|------|------|
| 仅复制 | 关闭 | 保留源卡文件，只复制到目标目录 |
| 试运行 | 关闭 | 只输出计划，不移动、不复制 |

## 📁 归档结构

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

## 🗂️ 项目结构

```text
src/
├── main.rs        程序入口、窗口初始化
├── app.rs         界面状态、后台归档线程、日志
├── lib.rs         模块导出与默认扩展名
├── detector.rs    挂载点枚举、RAW 特征探测
├── metadata.rs    EXIF 拍摄时间、mtime 回退、目录名
├── organizer.rs   文件收集、去重命名、跨盘移动、xmp 补迁
└── xmp.rs         .xmp 路径处理
tests/
└── integration_test.rs
```

## ⚠️ 注意事项

- 归档是移动操作，默认会删除源卡文件。首次使用建议先勾选「试运行」确认计划。
- 自动探测依赖挂载点约定。存储卡挂在非常规位置时可能识别不到，此时用「添加文件夹」手动指定。
- Windows 上除系统盘外的所有盘只要存在 RAW 文件都会被列为候选，包括你已经拷贝过照片的数据盘。
- 界面为中文，缺少系统中文字体时文字会显示为方框。

## 🛠️ 开发指南

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

release 配置启用了体积优化（`opt-level = "z"`、LTO、`strip`），Windows 下可执行文件约 5.7 MB。

## 🙏 致谢

- [eframe / egui](https://github.com/emilk/egui) — 窗口界面
- [nom-exif](https://github.com/mindeng/nom-exif) — EXIF 解析
- [sysinfo](https://github.com/GuillaumeGomez/sysinfo) — 挂载点枚举
- [walkdir](https://github.com/BurntSushi/walkdir) — 目录遍历
- [rfd](https://github.com/PolyMeilex/rfd) — 系统文件选择对话框

## 📄 License

[MIT](LICENSE)
