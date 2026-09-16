//! Shared test-only process startup coordination.
//!
//! Real-listener integration tests must release their ephemeral `TcpListener` reservations before
//! the compiled gateway can bind the same addresses. Cargo may execute different integration-test
//! binaries as separate processes, so an in-process mutex cannot protect that handoff globally.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(90);
const RETRY_INTERVAL: Duration = Duration::from_millis(25);

/// Cross-process guard for the short reservation-release -> child-readiness handoff.
pub(crate) struct StartupLock {
    _file: File,
}

impl StartupLock {
    /// Acquires the repository-wide test startup lock without adding a runtime dependency.
    pub(crate) fn acquire() -> Self {
        let path = std::env::temp_dir().join("cwl-pingora-gateway-test-startup-v2.lock");
        Self::acquire_at(&path, ACQUIRE_TIMEOUT)
    }

    /// Acquires one test-scoped startup lock with a caller-supplied bounded deadline.
    pub(crate) fn acquire_at(path: &Path, timeout: Duration) -> Self {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to open cross-process gateway startup lock at {}: {error}",
                    path.display()
                )
            });
        let deadline = Instant::now() + timeout;

        loop {
            match file.try_lock() {
                Ok(()) => return Self { _file: file },
                Err(TryLockError::WouldBlock) => {
                    assert!(
                        Instant::now() < deadline,
                        "timed out waiting for the cross-process gateway startup lock at {}",
                        path.display()
                    );
                    thread::sleep(RETRY_INTERVAL);
                }
                Err(TryLockError::Error(error)) => panic!(
                    "failed to acquire cross-process gateway startup lock at {}: {error}",
                    path.display()
                ),
            }
        }
    }
}
