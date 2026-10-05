# Release helper: bump the version, commit, tag and push.
# Pushing the tag triggers the GitHub Actions release workflow.
#
# Usage:
#   pwsh scripts/release.ps1            # patch (default)
#   pwsh scripts/release.ps1 minor
#   pwsh scripts/release.ps1 major
#
# Uses your existing git credentials; no GitHub token required.

param(
    [Parameter(Position = 0)]
    [ValidateSet("patch", "minor", "major")]
    [string]$Bump = "patch"
)

$ErrorActionPreference = "Stop"

Set-Location (git rev-parse --show-toplevel)

$cargoPath = "Cargo.toml"
$content = [System.IO.File]::ReadAllText($cargoPath)
$match = [regex]::Match($content, '(?m)^version\s*=\s*"(\d+)\.(\d+)\.(\d+)"')
if (-not $match.Success) {
    throw "Cargo.toml does not contain a X.Y.Z version line"
}

$major = [int]$match.Groups[1].Value
$minor = [int]$match.Groups[2].Value
$patch = [int]$match.Groups[3].Value
$current = "$major.$minor.$patch"

switch ($Bump) {
    "major" { $major++; $minor = 0; $patch = 0 }
    "minor" { $minor++; $patch = 0 }
    default { $patch++ }
}

$version = "$major.$minor.$patch"
$tag = "v$version"

Write-Host "Release bump ($Bump): $current -> $version"

$updated = [regex]::Replace($content, '(?m)^version\s*=\s*"\d+\.\d+\.\d+"', "version = `"$version`"", 1)
[System.IO.File]::WriteAllText((Resolve-Path $cargoPath), $updated)

Write-Host "Building release (also refreshes Cargo.lock)..."
cargo build --release

git add Cargo.toml Cargo.lock
git commit -m "chore(release): $tag"
git tag $tag
git push
git push origin $tag

Write-Host ""
Write-Host "Pushed $tag. GitHub Actions will build and publish the release."
Write-Host "Watch: https://github.com/WuCaiCaiCai/CamDrop/actions"
