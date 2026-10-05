# <img src="assets/icon.svg" width="36" alt=""> CamDrop

[![GitHub stars](https://img.shields.io/github/stars/WuCaiCaiCai/CamDrop?style=social)](https://github.com/WuCaiCaiCai/CamDrop/stargazers)
[![Release](https://img.shields.io/github/v/release/WuCaiCaiCai/CamDrop?sort=semver)](https://github.com/WuCaiCaiCai/CamDrop/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.94%2B-orange.svg?logo=rust&logoColor=white)](https://www.rust-lang.org)

[English](README_EN.md)

CamDrop 是一个相机存储卡迁移工具。插入存储卡后，它会扫描已挂载的磁盘找到相机卡，读取每张照片的拍摄时间，把照片和视频按 `年/月_日` 分目录，移动到指定位置，并带上同名的 `.xmp` 侧边栏文件。用 Rust 编写，编译为单个可执行文件，附带一个基于 egui 的简单界面。

[功能](#功能) • [安装](#安装) • [使用](#使用) • [开发](#开发) • [许可](#许可)

## 功能

* 自动识别存储卡：扫描已挂载磁盘，依据 `.nef` `.cr3` `.arw` `.dng` 等 RAW 文件判断相机卡。
* 按拍摄时间迁移：读取 EXIF 的 `DateTimeOriginal` 建立目录，读不到时退回文件修改时间。
* 同步 `.xmp`：照片移动时把同名侧边栏文件一并搬走，搬完后再次扫描并补迁遗漏的 xmp。
* 跨盘安全移动：`rename` 遇到 CrossesDevices 时自动降级为「复制 + 删除」。
* 只读文件处理：删除失败时先清除只读属性再重试。
* 重名保护：目标已有同名文件时追加 `_1`、`_2`，不覆盖。
* 时间筛选：在年/月/日列表里勾选要迁移的时间，预览实时同步。
* 移动或复制：两种方式二选一，移动会删除源卡文件，复制则保留。

迁移支持 `.jpg` `.jpeg` `.nef` `.cr3` `.arw` `.dng` `.mp4` `.mov`；探测还识别 `.raf` `.orf` `.rw2` `.pef` `.srw` `.nrw`。

## 安装

### 下载预编译版本

前往 [Releases](https://github.com/WuCaiCaiCai/CamDrop/releases) 下载对应系统的可执行文件并运行。

### 从源码构建

需要 Rust 1.94 或更新的 stable 工具链。

```bash
git clone https://github.com/WuCaiCaiCai/CamDrop.git
cd CamDrop
cargo build --release
```

产物位于 `target/release/`：Linux / macOS 是 `camdrop`，Windows 是 `camdrop.exe`。

### 使用 cargo 安装

```bash
cargo install --git https://github.com/WuCaiCaiCai/CamDrop.git
```

### 运行环境

* 操作系统：Windows、Linux 或 macOS。
* 图形接口：OpenGL，由显卡驱动提供。
* 中文字体：界面为中文，启动时会从系统字体中查找（Windows 微软雅黑、Linux Noto CJK 或文泉驿、macOS 苹方），字体不打包进程序。

## 使用

运行程序后：

1. 在左侧「选择来源」里勾选作为来源的相机卡；没有识别到时点「添加文件夹」手动选择。
2. 点击「扫描所选来源」，中央区域会列出文件预览和目录预览。
3. 确认目标目录（默认为程序目录下的 `RAW`），选择「移动」或「复制」，并在「按时间筛选」里勾选要迁移的年/月/日。
4. 点击「开始迁移」，在底部日志区查看每个文件的处理结果。

「移动」会删除源卡上的原文件；「复制」保留源文件，只向目标目录复制一份。时间筛选只影响本次迁移的范围，上方的预览会同步更新。

预览有「文件」和「目录预览」两个视图：前者逐个列出文件名、大小、拍摄时间和迁移到的位置，后者按目标目录分组展示迁移后的结构。

### 目录结构

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

### 平台差异

| 平台 | 探测范围 |
|------|----------|
| Windows | 除系统盘外的所有盘符 |
| Linux | `/media`、`/mnt`、`/run/media` |
| macOS | `/Volumes` |

自动探测依赖这些挂载约定，存储卡挂在其他位置时用「添加文件夹」手动指定。

## 开发

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

release 配置启用了体积优化（`opt-level = "z"`、LTO、`strip`），Windows 下可执行文件约 5.7 MB。

## 项目结构

```text
src/
├── main.rs        程序入口、窗口初始化
├── app.rs         界面状态、后台迁移线程、日志
├── lib.rs         模块导出与默认扩展名
├── detector.rs    挂载点枚举、RAW 特征探测
├── metadata.rs    EXIF 拍摄时间、mtime 回退、目录名
├── organizer.rs   文件收集、去重命名、跨盘移动、xmp 补迁
└── xmp.rs         .xmp 路径处理
tests/
└── integration_test.rs
```

## 注意

* 「移动」默认删除源卡上的原文件，首次使用建议先「扫描」核对预览，或选择「复制」。
* Windows 上除系统盘外的数据盘只要含 RAW 文件也会被列为候选。
* 缺少系统中文字体时界面文字会显示为方框。

## 致谢

* [eframe / egui](https://github.com/emilk/egui) — 窗口界面
* [egui_extras](https://github.com/emilk/egui/tree/master/crates/egui_extras) — 表格控件
* [nom-exif](https://github.com/mindeng/nom-exif) — EXIF 解析
* [sysinfo](https://github.com/GuillaumeGomez/sysinfo) — 挂载点枚举
* [walkdir](https://github.com/BurntSushi/walkdir) — 目录遍历
* [rfd](https://github.com/PolyMeilex/rfd) — 系统文件选择对话框

## 许可

CamDrop 以 [MIT](LICENSE) 许可证发布。
