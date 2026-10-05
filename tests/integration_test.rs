use std::fs;

use camdrop::organizer::{self, Options};
use tempfile::tempdir;

#[test]
fn collect_files_filters_and_skips_hidden() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.NEF"), b"x").unwrap();
    fs::write(dir.path().join("b.txt"), b"x").unwrap();
    fs::write(dir.path().join("._c.JPG"), b"x").unwrap();

    let files = organizer::collect_files(dir.path(), &["nef", "jpg"]);
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("a.NEF"));
}

#[test]
fn unique_path_appends_counter() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("a.NEF");
    fs::write(&target, b"x").unwrap();

    let unique = organizer::unique_path(&target);
    assert_eq!(unique.file_name().unwrap(), "a_1.NEF");
}

#[test]
fn safe_move_moves_file() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.NEF");
    let dst = dir.path().join("sub").join("dst.NEF");
    fs::write(&src, b"data").unwrap();

    organizer::safe_move(&src, &dst).unwrap();

    assert!(!src.exists());
    assert_eq!(fs::read(&dst).unwrap(), b"data");
}

#[test]
fn reconcile_moves_orphan_xmp() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();

    let photo_dir = dst.path().join("2026").join("09_24");
    fs::create_dir_all(&photo_dir).unwrap();
    fs::write(photo_dir.join("DSC_0001.NEF"), b"raw").unwrap();
    fs::write(src.path().join("DSC_0001.NEF.xmp"), b"<xmp/>").unwrap();

    let count = organizer::reconcile_orphans(src.path(), dst.path());

    assert_eq!(count, 1);
    assert!(photo_dir.join("DSC_0001.NEF.xmp").exists());
}

#[test]
fn organize_dry_run_keeps_source() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    fs::write(src.path().join("a.NEF"), b"raw").unwrap();

    let opts = Options {
        copy_only: false,
        dry_run: true,
    };
    let summary = organizer::organize(&[src.path().to_path_buf()], dst.path(), &opts, |_| {});

    assert_eq!(summary.moved, 0);
    assert!(src.path().join("a.NEF").exists());
}
