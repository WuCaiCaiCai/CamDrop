//! Safe file movement and the date-based archival pipeline.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::metadata;
use crate::xmp;
use crate::{DEFAULT_EXTENSIONS, has_extension};

/// Archival options.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Copy instead of moving the source files.
    pub copy_only: bool,
    /// Print the plan without touching any file.
    pub dry_run: bool,
}

/// Result counters for one archival run.
#[derive(Debug, Clone, Copy, Default)]
pub struct Summary {
    pub moved: usize,
    pub xmp: usize,
    pub orphans: usize,
    pub errors: usize,
}

/// Progress events emitted during [`organize`].
pub enum Event {
    Log(String),
    Progress(usize, usize),
}

/// Collects supported media files under `root`.
///
/// macOS `._` sidecar files are ignored.
pub fn collect_files(root: &Path, exts: &[&str]) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| !e.file_name().to_string_lossy().starts_with("._"))
        .map(walkdir::DirEntry::into_path)
        .filter(|p| has_extension(p, exts))
        .collect()
}

fn collect_all(sources: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for source in sources {
        files.extend(collect_files(source, DEFAULT_EXTENSIONS));
    }
    files
}

/// One row of the archive preview.
#[derive(Debug, Clone)]
pub struct PreviewItem {
    pub source: PathBuf,
    pub size: u64,
    pub captured: String,
    pub folder: String,
    pub has_xmp: bool,
}

/// Resolves capture time and target folder for every file, without moving anything.
pub fn preview(sources: &[PathBuf], mut on_event: impl FnMut(Event)) -> Vec<PreviewItem> {
    let files = collect_all(sources);
    let total = files.len();
    on_event(Event::Progress(0, total));

    let mut items = Vec::with_capacity(total);
    for (index, file) in files.iter().enumerate() {
        let dt = metadata::capture_time(file);
        let captured = dt
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "未知".to_owned());
        let folder = dt
            .map(|dt| metadata::date_dir(&dt).to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| "unknown".to_owned());
        let size = fs::metadata(file).map(|m| m.len()).unwrap_or(0);

        items.push(PreviewItem {
            source: file.clone(),
            size,
            captured,
            folder,
            has_xmp: xmp::sidecar_of(file).is_file(),
        });
        on_event(Event::Progress(index + 1, total));
    }
    items
}

/// Returns a destination path that does not collide with an existing file.
pub fn unique_path(dst: &Path) -> PathBuf {
    if !dst.exists() {
        return dst.to_path_buf();
    }
    let parent = dst.parent().unwrap_or_else(|| Path::new("."));
    let stem = dst
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = dst.extension().map(|e| e.to_string_lossy().into_owned());

    let mut counter = 1;
    loop {
        let name = match &ext {
            Some(ext) => format!("{stem}_{counter}.{ext}"),
            None => format!("{stem}_{counter}"),
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

/// Moves `src` to `dst`, falling back to copy+delete across devices.
pub fn safe_move(src: &Path, dst: &Path) -> io::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(e) if is_cross_device(&e) => {
            fs::copy(src, dst)?;
            remove_file_force(src)
        }
        Err(e) => Err(e),
    }
}

fn copy_file(src: &Path, dst: &Path) -> io::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src, dst)?;
    Ok(())
}

#[allow(clippy::permissions_set_readonly_false)]
fn remove_file_force(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(_) => {
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_readonly(false);
            fs::set_permissions(path, perms)?;
            fs::remove_file(path)
        }
    }
}

fn is_cross_device(e: &io::Error) -> bool {
    if e.kind() == io::ErrorKind::CrossesDevices {
        return true;
    }
    matches!(e.raw_os_error(), Some(code) if is_exdev(code))
}

#[cfg(windows)]
fn is_exdev(code: i32) -> bool {
    code == 17
}

#[cfg(not(windows))]
fn is_exdev(code: i32) -> bool {
    code == 18
}

/// Archives every supported file from `sources` into `target/<year>/<MM_dd>/`.
///
/// Sidecar `.xmp` files follow their photo; leftover sidecars are reconciled
/// against already-migrated photos at the end.
pub fn organize(
    sources: &[PathBuf],
    target: &Path,
    opts: &Options,
    mut on_event: impl FnMut(Event),
) -> Summary {
    let files = collect_all(sources);
    let total = files.len();
    on_event(Event::Progress(0, total));

    let mut summary = Summary::default();

    for (index, file) in files.iter().enumerate() {
        let sub = metadata::capture_time(file)
            .map(|dt| metadata::date_dir(&dt))
            .unwrap_or_else(|| PathBuf::from("unknown"));
        let dir = target.join(sub);
        let file_name = file.file_name().unwrap_or_default();

        if opts.dry_run {
            let dst = dir.join(file_name);
            on_event(Event::Log(format!(
                "[试运行] {} -> {}",
                file.display(),
                dst.display()
            )));
            on_event(Event::Progress(index + 1, total));
            continue;
        }

        let dst = unique_path(&dir.join(file_name));
        let result = if opts.copy_only {
            copy_file(file, &dst)
        } else {
            safe_move(file, &dst)
        };

        match result {
            Ok(()) => {
                summary.moved += 1;
                on_event(Event::Log(format!(
                    "{} -> {}",
                    file.display(),
                    dst.display()
                )));

                let sidecar = xmp::sidecar_of(file);
                if sidecar.is_file() {
                    let sidecar_dst = xmp::sidecar_of(&dst);
                    if safe_move(&sidecar, &sidecar_dst).is_ok() {
                        summary.xmp += 1;
                    }
                }
            }
            Err(e) => {
                summary.errors += 1;
                on_event(Event::Log(format!("失败: {} ({e})", file.display())));
            }
        }

        on_event(Event::Progress(index + 1, total));
    }

    if !opts.dry_run {
        for source in sources {
            summary.orphans += reconcile_orphans(source, target);
        }
    }

    summary
}

/// Moves leftover `.xmp` files next to their already-migrated photos.
pub fn reconcile_orphans(src_root: &Path, dst_root: &Path) -> usize {
    let mut migrated: HashMap<String, PathBuf> = HashMap::new();
    for entry in WalkDir::new(dst_root).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file()
            && let Some(name) = entry.file_name().to_str()
        {
            migrated.insert(name.to_ascii_lowercase(), entry.path().to_path_buf());
        }
    }

    let mut count = 0;
    for entry in WalkDir::new(src_root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        let Some(photo_name) = lower.strip_suffix(".xmp") else {
            continue;
        };

        let Some(photo) = migrated.get(photo_name) else {
            continue;
        };
        let sidecar_dst = xmp::sidecar_of(photo);
        if sidecar_dst.exists() {
            continue;
        }
        if safe_move(path, &sidecar_dst).is_ok() {
            count += 1;
        }
    }
    count
}
