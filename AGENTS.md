# AGENTS.md

Guidance for AI agents working in this repository.

## Project

CamDrop — a native desktop (Rust + eframe/egui) tool that migrates camera memory
card photos/videos into date folders and keeps `.xmp` sidecars together. See
[PRODUCT.md](PRODUCT.md) for product truth and [DESIGN.md](DESIGN.md) for the
design system and hard rules.

## Ground rules

- Read [DESIGN.md](DESIGN.md) before changing UI. It lists non-negotiable rules
  (contrast/legibility, accent usage) and the egui pitfalls that have caused bugs
  before (`strong_text_color`, nested `Panel` in a resizable side panel, content
  wider than the side panel).
- Keep changes compiling and green: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `cargo build --release`.

## Releasing

When the user says something like "更新版本并发版" / "release a new version" /
"bump and publish", with an optional bump kind (patch / minor / major, default patch):

1. Trigger the release workflow:
   ```
   gh workflow run release.yml -f bump=patch     # or minor / major
   ```
2. Follow it and wait for completion:
   ```
   gh run watch
   ```
3. Report the release URL to the user (`gh release view --web` or the printed link).

The workflow (`.github/workflows/release.yml`) does everything else: it bumps the
version in `Cargo.toml`/`Cargo.lock`, commits and tags `vX.Y.Z`, builds Windows /
Linux / macOS binaries, produces `.deb`, `.rpm`, `.AppImage` and `.tar.gz`/`.zip`,
and publishes a GitHub Release with auto-generated notes.

Full details, prerequisites, and manual fallbacks: [RELEASING.md](RELEASING.md).
