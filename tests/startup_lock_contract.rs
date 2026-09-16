mod support;

use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;

const STARTUP_LOCK_FILE: &str = "cwl-pingora-gateway-test-startup-v2.lock";

fn open_lock_file(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .expect("open startup-lock contract file")
}

#[test]
fn startup_lock_excludes_a_competing_file_handle_and_releases_on_drop() {
    let startup_lock = support::StartupLock::acquire();
    let path = std::env::temp_dir().join(STARTUP_LOCK_FILE);
    let contender = open_lock_file(&path);

    assert!(matches!(
        contender.try_lock(),
        Err(TryLockError::WouldBlock)
    ));

    drop(startup_lock);
    contender
        .try_lock()
        .expect("dropping the owner must release the operating-system file lock");
    contender.unlock().expect("release contender lock");
}

#[test]
fn dropping_an_old_unlocked_handle_cannot_release_a_successor_lock() {
    let tempdir = tempfile::tempdir().expect("create lock-contract tempdir");
    let path = tempdir.path().join("startup.lock");
    let predecessor = open_lock_file(&path);
    predecessor.try_lock().expect("lock predecessor handle");
    predecessor.unlock().expect("release predecessor lock");

    let successor = open_lock_file(&path);
    successor.try_lock().expect("lock successor handle");
    drop(predecessor);

    let contender = open_lock_file(&path);
    assert!(matches!(
        contender.try_lock(),
        Err(TryLockError::WouldBlock)
    ));

    successor.unlock().expect("release successor lock");
}
