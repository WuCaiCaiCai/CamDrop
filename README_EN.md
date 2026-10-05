# CamDrop

[![GitHub stars](https://img.shields.io/github/stars/WuCaiCaiCai/CamDrop?style=social)](https://github.com/WuCaiCaiCai/CamDrop/stargazers)
[![Release](https://img.shields.io/github/v/release/WuCaiCaiCai/CamDrop?sort=semver)](https://github.com/WuCaiCaiCai/CamDrop/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.94%2B-orange.svg?logo=rust&logoColor=white)](https://www.rust-lang.org)

[简体中文](README.md)

CamDrop is a camera memory card migration tool. After you insert a card, it scans the mounted disks to find the card, reads each photo's capture time, sorts the photos and videos into `year/month_day` folders, moves them to the destination, and takes the matching `.xmp` sidecars along. It is written in Rust, compiles to a single executable, and ships with a small egui interface.

[Features](#features) • [Installation](#installation) • [Usage](#usage) • [Development](#development) • [License](#license)

## Features

* Automatic card detection: scans mounted disks and identifies the card by RAW files such as `.nef`, `.cr3`, `.arw`, `.dng`.
* Date-based migration: uses EXIF `DateTimeOriginal` to build the folder, falling back to the file modification time.
* XMP sync: moves the matching `.xmp` with each photo, then rescans the card and re-matches any leftover sidecars.
* Safe cross-drive moves: falls back to copy + delete when `rename` returns CrossesDevices.
* Read-only handling: clears the read-only attribute and retries when deletion fails.
* Collision handling: appends `_1`, `_2` instead of overwriting an existing file.
* Date filter: migrate only the files of a chosen year, month or day; the preview updates with it.
* Move or copy: pick one, where move deletes the card files and copy keeps them.

Migration handles `.jpg` `.jpeg` `.nef` `.cr3` `.arw` `.dng` `.mp4` `.mov`; detection also recognises `.raf` `.orf` `.rw2` `.pef` `.srw` `.nrw`.

## Installation

### Download a prebuilt binary

Get the binary for your platform from the [Releases](https://github.com/WuCaiCaiCai/CamDrop/releases) page and run it.

### Build from source

Rust 1.94 or newer stable is required.

```bash
git clone https://github.com/WuCaiCaiCai/CamDrop.git
cd CamDrop
cargo build --release
```

The output is under `target/release/`: `camdrop` on Linux / macOS, `camdrop.exe` on Windows.

### Install with cargo

```bash
cargo install --git https://github.com/WuCaiCaiCai/CamDrop.git
```

### Requirements

* OS: Windows, Linux or macOS.
* Graphics: OpenGL, provided by the GPU driver.
* CJK font: the UI is in Chinese and looks for a system font at startup (Microsoft YaHei on Windows, Noto CJK or wqy on Linux, PingFang on macOS). The font is not bundled.

## Usage

After launching the program:

1. Select the source cards under "Select source"; click "Add folder" if none was detected.
2. Click "Scan selected"; the center area lists the file preview and the destination layout.
3. Confirm the destination folder (defaults to `RAW` next to the executable), pick "Move" or "Copy", and narrow the files with the date filter (all / year / month / day).
4. Click "Start migration" and watch the log at the bottom for the result of each file.

"Move" deletes the original files from the card; "Copy" keeps them and writes a second set to the destination. The date filter only changes the scope of this run, and the preview follows it.

The preview has two views. "Files" lists each file's name, size, capture time and destination path; "Destination layout" groups the files by their target folder.

### Destination layout

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

### Platform notes

| Platform | Detection scope |
|----------|-----------------|
| Windows | Every drive except the system drive |
| Linux | `/media`, `/mnt`, `/run/media` |
| macOS | `/Volumes` |

Detection relies on these mount conventions. If your card is mounted elsewhere, use "Add folder" to pick it manually.

## Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

The release profile enables size optimizations (`opt-level = "z"`, LTO, `strip`). The Windows binary is about 5.7 MB.

## Project layout

```text
src/
├── main.rs        Entry point and window setup
├── app.rs         UI state, background migration thread, log
├── lib.rs         Module exports and default extensions
├── detector.rs    Mount enumeration and RAW signature detection
├── metadata.rs    EXIF capture time, mtime fallback, folder name
├── organizer.rs   File collection, dedup naming, cross-drive moves, xmp reconciliation
└── xmp.rs         .xmp path helpers
tests/
└── integration_test.rs
```

## Notes

* "Move" removes the source files from the card by default, so scan and check the preview first, or choose "Copy".
* On Windows, data disks that contain RAW files are also listed as candidates.
* Without a Chinese font installed, the UI text shows as boxes.

## Credits

* [eframe / egui](https://github.com/emilk/egui) — window and UI
* [nom-exif](https://github.com/mindeng/nom-exif) — EXIF parsing
* [sysinfo](https://github.com/GuillaumeGomez/sysinfo) — mount enumeration
* [walkdir](https://github.com/BurntSushi/walkdir) — directory traversal
* [rfd](https://github.com/PolyMeilex/rfd) — native file dialogs

## License

CamDrop is released under the [MIT](LICENSE) license.
