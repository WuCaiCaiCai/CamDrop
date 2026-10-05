//! Capture-time resolution with EXIF parsing and modification-time fallback.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use nom_exif::{ExifDateTime, ExifTag};

/// Returns the capture time of `path`.
///
/// Tries `EXIF DateTimeOriginal` first; falls back to the file modification
/// time when EXIF is missing or unreadable.
pub fn capture_time(path: &Path) -> Option<DateTime<Local>> {
    if let Some(naive) = exif_time(path)
        && let Some(dt) = local_from_naive(naive)
    {
        return Some(dt);
    }
    file_mtime(path)
}

fn exif_time(path: &Path) -> Option<NaiveDateTime> {
    let exif = nom_exif::read_exif(path).ok()?;
    let value = exif.get(ExifTag::DateTimeOriginal)?;
    match value.as_datetime()? {
        ExifDateTime::Aware(dt) => Some(dt.naive_local()),
        ExifDateTime::Naive(dt) => Some(dt),
    }
}

fn file_mtime(path: &Path) -> Option<DateTime<Local>> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(DateTime::<Local>::from(modified))
}

fn local_from_naive(naive: NaiveDateTime) -> Option<DateTime<Local>> {
    Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| Local.from_local_datetime(&naive).earliest())
}

/// Builds the archive sub-directory for a capture time, e.g. `2026/09_24`.
pub fn date_dir(dt: &DateTime<Local>) -> PathBuf {
    PathBuf::from(dt.format("%Y").to_string()).join(dt.format("%m_%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn builds_year_then_month_day() {
        let dt = Local.with_ymd_and_hms(2026, 9, 24, 10, 30, 0).unwrap();
        assert_eq!(date_dir(&dt), PathBuf::from("2026").join("09_24"));
    }
}
