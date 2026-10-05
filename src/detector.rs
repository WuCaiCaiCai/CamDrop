//! Mount-point enumeration and RAW-signature based camera card detection.

use std::path::{Path, PathBuf};

use sysinfo::{Disk, Disks};
use walkdir::WalkDir;

use crate::has_extension;

/// RAW extensions used as a camera signature.
pub const RAW_SIGNATURES: &[&str] = &[
    "nef", "cr3", "arw", "dng", "raf", "orf", "rw2", "pef", "srw", "nrw",
];

/// Scans mounted disks and returns roots that look like camera cards.
///
/// Fixed internal drives are skipped, `DCIM` is preferred as the scan root, and
/// a shallow walk (depth 4) confirms a RAW signature is present.
pub fn detect_cards() -> Vec<PathBuf> {
    let system = std::env::var("SystemDrive").ok();
    let disks = Disks::new_with_refreshed_list();
    let mut cards = Vec::new();

    for disk in disks.list() {
        let mount = disk.mount_point();
        if !is_removable(disk) || !is_candidate_mount(mount, system.as_deref()) {
            continue;
        }
        let root = candidate_root(mount);
        if looks_like_camera(&root) {
            cards.push(root);
        }
    }
    cards
}

/// Only removable media counts as a camera card, so built-in drives are ignored.
#[cfg(windows)]
fn is_removable(disk: &Disk) -> bool {
    disk.is_removable()
}

#[cfg(not(windows))]
fn is_removable(_disk: &Disk) -> bool {
    true
}

fn candidate_root(mount: &Path) -> PathBuf {
    let dcim = mount.join("DCIM");
    if dcim.is_dir() {
        dcim
    } else {
        mount.to_path_buf()
    }
}

/// Shallow-walks `root` looking for any RAW file.
pub fn looks_like_camera(root: &Path) -> bool {
    WalkDir::new(root)
        .max_depth(4)
        .into_iter()
        .filter_map(Result::ok)
        .any(|e| e.file_type().is_file() && has_extension(e.path(), RAW_SIGNATURES))
}

/// Returns `true` if `mount` may host a removable camera card.
///
/// On Windows every drive except the system drive is considered. On Unix only
/// the conventional removable-media mount points are used, so the root
/// filesystem and home directories are never scanned.
#[cfg(windows)]
fn is_candidate_mount(mount: &Path, system: Option<&str>) -> bool {
    let Some(system) = system else {
        return true;
    };
    !mount
        .to_string_lossy()
        .to_uppercase()
        .starts_with(&system.to_uppercase())
}

#[cfg(not(windows))]
fn is_candidate_mount(mount: &Path, _system: Option<&str>) -> bool {
    const EXACT: &[&str] = &["/media", "/mnt", "/run/media", "/Volumes"];
    const PREFIXES: &[&str] = &["/media/", "/mnt/", "/run/media/", "/Volumes/"];
    let mount = mount.to_string_lossy();
    EXACT.contains(&mount.as_ref()) || PREFIXES.iter().any(|prefix| mount.starts_with(prefix))
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;

    #[test]
    fn accepts_removable_mounts_only() {
        assert!(is_candidate_mount(Path::new("/media/user/CARD"), None));
        assert!(is_candidate_mount(Path::new("/mnt/card"), None));
        assert!(is_candidate_mount(Path::new("/run/media/user/CARD"), None));
        assert!(!is_candidate_mount(Path::new("/"), None));
        assert!(!is_candidate_mount(Path::new("/home/user"), None));
        assert!(!is_candidate_mount(Path::new("/boot/efi"), None));
    }
}
