# TODO

Remaining work for CamDrop. Check items off as they are done.

## Release / CI

- [ ] Verify the tag-push release flow: run `pwsh scripts/release.ps1 patch` and confirm
      GitHub Actions triggers automatically (the `bump` job is skipped; `meta` / `build` /
      `release` run) without duplicating `v0.1.1`.
- [ ] Check the `v0.1.1` Release artifacts (`camdrop-*-windows-x86_64.zip`,
      `camdrop-*-macos-*.tar.gz`, `camdrop_0.1.1_amd64.deb`, `camdrop-0.1.1-1.x86_64.rpm`,
      `CamDrop-0.1.1-x86_64.AppImage`, `camdrop-*-linux-x86_64.tar.gz`). If any packaging
      step failed, read the Actions log and fix `.github/workflows/release.yml`. The Linux
      deb/rpm/AppImage steps have not been verified yet (first run was still in progress).
- [ ] Confirm the repository Actions workflow permissions are set to **Read and write**
      (required to create the Release).
- [ ] Drop macOS x86_64 (Intel) support: remove the `macos-13` / `x86_64-apple-darwin`
      entry from the build matrix in `.github/workflows/release.yml`, leaving Apple
      Silicon only (`macos-14` / `aarch64-apple-darwin`). Update README / RELEASING docs
      to reflect macOS arm64 only.

## UI

- [ ] Confirm the left panel layout with the user: the "开始迁移" button pinned at the
      bottom (explicit footer area), each year a full-width collapsible box, and the
      全选 / 全月 controls right-aligned on their rows.

## Privacy (decided)

- [x] Scrub the personal drive paths from `move.py` (done).
- Decided **not** to rewrite git history: the `2580533035@qq.com` commit author email and
  the old `move.py` paths stay as-is. No tokens/secrets exist in the repo.
