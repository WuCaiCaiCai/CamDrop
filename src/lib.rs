//! CamDrop core library.
//!
//! Detects camera SD cards, reads capture time, and archives media by date
//! while keeping `.xmp` sidecar files in sync.

pub mod detector;
pub mod metadata;
pub mod organizer;
pub mod xmp;

use std::path::Path;

/// File extensions handled by default.
pub const DEFAULT_EXTENSIONS: &[&str] = &["jpg", "jpeg", "nef", "cr3", "arw", "dng", "mp4", "mov"];

/// Returns `true` if `path` has one of `exts` (case-insensitive, no dot).
pub fn has_extension(path: &Path, exts: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| exts.iter().any(|x| x.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}
