//! Sidecar (`.xmp`) path helpers.

use std::path::{Path, PathBuf};

/// Returns the `.xmp` sidecar path for a media file, e.g. `a.NEF.xmp`.
pub fn sidecar_of(media: &Path) -> PathBuf {
    let mut name = media.as_os_str().to_os_string();
    name.push(".xmp");
    PathBuf::from(name)
}
