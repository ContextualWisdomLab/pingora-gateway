//! Shared test-only process startup coordination.
//!
//! Real-listener integration tests must release their ephemeral `TcpListener` reservations before
//! the compiled gateway can bind the same addresses. Cargo may execute different integration-test
//! binaries as separate processes, so an in-process mutex cannot protect that handoff globally.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(90);
const STALE_AFTER: Duration = Duration::from_secs(60);
const RETRY_INTERVAL: Duration = Duration::from_millis(25);

/// Cross-process guard for the short reservation-release -> child-readiness handoff.
pub(crate) struct StartupLock {
    path: PathBuf,
}

impl StartupLock {
    /// Acquires the repository-wide test startup lock without adding a runtime dependency.
    pub(crate) fn acquire() -> Self {
        let path = std::env::temp_dir().join("cwl-pingora-gateway-test-startup-v1.lock");
        let deadline = Instant::now() + ACQUIRE_TIMEOUT;

        loop {
            match fs::create_dir(&path) {
                Ok(()) => return Self { path },
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    if lock_is_stale(&path) {
                        let _ = fs::remove_dir(&path);
                        continue;
                    }
                    assert!(
                        Instant::now() < deadline,
                        "timed out waiting for the cross-process gateway startup lock at {}",
                        path.display()
                    );
                    thread::sleep(RETRY_INTERVAL);
                }
                Err(error) => panic!(
                    "failed to acquire cross-process gateway startup lock at {}: {error}",
                    path.display()
                ),
            }
        }
    }
}

impl Drop for StartupLock {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.path);
    }
}

fn lock_is_stale(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    let modified = metadata.modified().unwrap_or(SystemTime::now());
    modified
        .elapsed()
        .map(|age| age > STALE_AFTER)
        .unwrap_or(false)
}
