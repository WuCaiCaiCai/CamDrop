# CamDrop

> A camera memory card archiver: finds the card automatically, sorts photos and videos into date folders, and moves Darktable / Lightroom `.xmp` sidecars along with them.

🌐 English | [简体中文](README.md)

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## 📖 Introduction

CamDrop is a small desktop tool written in Rust. After you insert a camera card, it scans the mounted disks, identifies the card by the RAW files it contains, then sorts the photos and videos into `year/month_day` folders and moves them to the destination. Any `.xmp` file with the same name as a photo is moved together with it.

The interface is built with eframe / egui. It compiles to a single executable with no Python or other runtime dependency.

## ✨ Features

### 🎯 Core

| Feature | Description |
|---------|-------------|
| Automatic card detection | Scans mounted disks and identifies the card by RAW files such as `.nef`, `.cr3`, `.arw`, `.dng`. Windows skips the system drive; Linux looks under `/media`, `/mnt` and `/run/media`; macOS looks under `/Volumes` |
| Date-based archiving | Uses EXIF `DateTimeOriginal` to build the folder; falls back to the file modification time when it is missing |
| XMP sync | Moves the matching `.xmp` with each photo, then rescans the card and re-matches any leftover sidecars by file name |
| Safe cross-drive moves | Falls back to copy + delete when `rename` returns CrossesDevices |
| Read-only handling | Clears the read-only attribute and retries when deletion fails |
| Collision handling | Appends `_1`, `_2` instead of overwriting an existing file |

### 🖥️ Interface

| Feature | Description |
|---------|-------------|
| Selectable source cards | Detected cards are listed and can be multi-selected; use "Add folder" when detection fails |
| Destination folder | Defaults to `RAW` next to the executable, changeable via "Browse" |
| Copy only | Keeps the card files and copies a second set to the destination |
| Dry run | Prints the archive plan without touching any file |
| Progress and log | Runs on a background thread, shows a progress bar and logs the result of every file, flagging failures |

### 🧩 Supported formats

| Type | Extensions |
|------|------------|
| Photos | `.jpg` `.jpeg` `.nef` `.cr3` `.arw` `.dng` |
| Videos | `.mp4` `.mov` |
| RAW signatures for detection | Besides the archive list, `.raf` `.orf` `.rw2` `.pef` `.srw` `.nrw` |

## 🚀 Quick Start

### Requirements

| Item | Requirement |
|------|-------------|
| OS | Windows / Linux / macOS |
| Graphics | OpenGL (provided by the GPU driver) |
| CJK font | A Chinese font must be installed: Microsoft YaHei on Windows, Noto CJK or wqy on Linux, PingFang on macOS (read from the system, not bundled) |
| Toolchain | Rust 1.94 or newer stable |

### Option 1: Download a prebuilt binary (recommended)

1. Open the [Releases](../../releases) page
2. Download the binary for your platform
3. Run it

### Option 2: Build from source

```bash
git clone https://github.com/WuCaiCaiCai/CamDrop.git
cd CamDrop
cargo build --release
```

The output is under `target/release/`: `camdrop` on Linux / macOS, `camdrop.exe` on Windows.

## 📖 Usage

### Steps

| Step | Action |
|------|--------|
| 1 | Run the program and wait for the scan to finish; detected cards are listed |
| 2 | Select the cards to archive, or click "Add folder" if none was detected |
| 3 | Confirm the destination folder (defaults to `RAW` next to the executable) |
| 4 | Toggle "Copy only" or "Dry run" as needed |
| 5 | Click "Start" and watch the log area |

### Options

| Option | Default | Description |
|--------|---------|-------------|
| Copy only | Off | Keep the card files and copy to the destination |
| Dry run | Off | Print the plan only, no move or copy |

## 📁 Archive layout

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

## 🗂️ Project layout

```text
src/
├── main.rs        Entry point and window setup
├── app.rs         UI state, background archive thread, log
├── lib.rs         Module exports and default extensions
├── detector.rs    Mount enumeration and RAW signature detection
├── metadata.rs    EXIF capture time, mtime fallback, folder name
├── organizer.rs   File collection, dedup naming, cross-drive moves, xmp reconciliation
└── xmp.rs         .xmp path helpers
tests/
└── integration_test.rs
```

## ⚠️ Notes

- Archiving moves files and removes them from the card by default. Use "Dry run" first if unsure.
- Detection relies on conventional mount points. A card mounted elsewhere may not be found; use "Add folder" in that case.
- On Windows, every disk except the system drive that contains RAW files is listed, including data disks you already copied from.
- The UI is Chinese. Without an installed Chinese font, text shows as boxes.

## 🛠️ Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

The release profile enables size optimizations (`opt-level = "z"`, LTO, `strip`). The Windows binary is about 5.7 MB.

## 🙏 Credits

- [eframe / egui](https://github.com/emilk/egui) — window and UI
- [nom-exif](https://github.com/mindeng/nom-exif) — EXIF parsing
- [sysinfo](https://github.com/GuillaumeGomez/sysinfo) — mount enumeration
- [walkdir](https://github.com/BurntSushi/walkdir) — directory traversal
- [rfd](https://github.com/PolyMeilex/rfd) — native file dialogs

## 📄 License

[MIT](LICENSE)
