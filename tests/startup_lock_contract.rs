mod support;

use std::env;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::path::Path;
use std::process::{self, Command};
use std::thread;
use std::time::{Duration, Instant};

const STARTUP_LOCK_FILE: &str = "cwl-pingora-gateway-test-startup-v2.lock";
const CHILD_LOCK_PATH: &str = "CWL_STARTUP_LOCK_CHILD_PATH";
const CHILD_READY_PATH: &str = "CWL_STARTUP_LOCK_CHILD_READY_PATH";

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

#[test]
fn abrupt_owner_process_exit_releases_lock_without_stale_lease_waiting() {
    let tempdir = tempfile::tempdir().expect("create process-exit tempdir");
    let lock_path = tempdir.path().join("startup.lock");
    let ready_path = tempdir.path().join("owner.ready");
    let executable = env::current_exe().expect("resolve current integration-test executable");
    let mut child = Command::new(executable)
        .arg("--exact")
        .arg("startup_lock_child_exits_without_drop")
        .arg("--nocapture")
        .env(CHILD_LOCK_PATH, &lock_path)
        .env(CHILD_READY_PATH, &ready_path)
        .spawn()
        .expect("spawn startup-lock child process");

    let ready_deadline = Instant::now() + Duration::from_secs(5);
    while !ready_path.exists() {
        if let Some(status) = child.try_wait().expect("poll startup-lock child") {
            panic!("startup-lock child exited before signalling ownership: {status}");
        }
        assert!(
            Instant::now() < ready_deadline,
            "timed out waiting for startup-lock child ownership signal"
        );
        thread::sleep(Duration::from_millis(10));
    }

    let contender = open_lock_file(&lock_path);
    assert!(matches!(
        contender.try_lock(),
        Err(TryLockError::WouldBlock)
    ));

    let status = child.wait().expect("wait for startup-lock child exit");
    assert!(status.success(), "startup-lock child failed: {status}");

    let recovered = support::StartupLock::acquire_at(&lock_path, Duration::from_secs(1));
    drop(recovered);
}

#[test]
fn continuously_held_lock_times_out_with_existing_diagnostic() {
    let tempdir = tempfile::tempdir().expect("create timeout-contract tempdir");
    let lock_path = tempdir.path().join("startup.lock");
    let holder = open_lock_file(&lock_path);
    holder.try_lock().expect("lock timeout-contract holder");

    let started = Instant::now();
    let result = std::panic::catch_unwind(|| {
        support::StartupLock::acquire_at(&lock_path, Duration::from_millis(100))
    });
    let elapsed = started.elapsed();
    let panic = result.expect_err("contended startup lock must fail after its bounded deadline");
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or("");

    assert!(
        message.contains("timed out waiting for the cross-process gateway startup lock"),
        "unexpected timeout diagnostic: {message}"
    );
    assert!(elapsed >= Duration::from_millis(100));
    assert!(
        elapsed < Duration::from_secs(5),
        "bounded startup-lock acquisition took {elapsed:?}"
    );

    holder.unlock().expect("release timeout-contract holder");
}

#[test]
fn startup_lock_child_exits_without_drop() {
    let Some(lock_path) = env::var_os(CHILD_LOCK_PATH) else {
        return;
    };
    let ready_path = env::var_os(CHILD_READY_PATH).expect("child ready path must be configured");
    let _lock = support::StartupLock::acquire_at(Path::new(&lock_path), Duration::from_secs(5));
    fs::write(Path::new(&ready_path), b"locked").expect("signal child lock ownership");
    thread::sleep(Duration::from_millis(250));

    process::exit(0);
}
