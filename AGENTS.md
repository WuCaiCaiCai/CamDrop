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
"bump and publish", with an optional bump kind (patch / minor / major, default patch),
run the local helper — it bumps the version, commits, tags and pushes; pushing the
tag triggers GitHub Actions to build and publish:

```
pwsh scripts/release.ps1 patch     # or minor / major
```

It uses the normal git credentials (no GitHub token). Then watch the run and report
the release URL to the user (the run page is printed by the helper; `gh run watch`
if `gh` is available).

The workflow (`.github/workflows/release.yml`) builds Windows / Linux / macOS binaries,
produces `.deb`, `.rpm`, `.AppImage` and `.tar.gz`/`.zip`, and publishes a GitHub Release
with auto-generated notes. A manual `workflow_dispatch` fallback also exists, but it
needs GitHub API auth — prefer the tag-push flow.

Full details, prerequisites, and manual fallbacks: [RELEASING.md](RELEASING.md).
