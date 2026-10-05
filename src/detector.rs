//! Mount-point enumeration and RAW-signature based camera card detection.

use std::path::{Path, PathBuf};

use sysinfo::Disks;
use walkdir::WalkDir;

use crate::has_extension;

/// RAW extensions used as a camera signature.
pub const RAW_SIGNATURES: &[&str] = &[
    "nef", "cr3", "arw", "dng", "raf", "orf", "rw2", "pef", "srw", "nrw",
];

/// Scans mounted disks and returns roots that look like camera cards.
///
/// System drives are skipped, `DCIM` is preferred as the scan root, and a
/// shallow walk (depth 4) confirms a RAW signature is present.
pub fn detect_cards() -> Vec<PathBuf> {
    let system = std::env::var("SystemDrive").ok();
    let disks = Disks::new_with_refreshed_list();
    let mut cards = Vec::new();

    for disk in disks.list() {
        let mount = disk.mount_point();
        if is_system_drive(mount, system.as_deref()) {
            continue;
        }
        let root = candidate_root(mount);
        if looks_like_camera(&root) {
            cards.push(root);
        }
    }
    cards
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

#[cfg(windows)]
fn is_system_drive(mount: &Path, system: Option<&str>) -> bool {
    let Some(system) = system else {
        return false;
    };
    let mount = mount.to_string_lossy().to_uppercase();
    mount.starts_with(&system.to_uppercase())
}

#[cfg(not(windows))]
fn is_system_drive(_mount: &Path, _system: Option<&str>) -> bool {
    false
}
