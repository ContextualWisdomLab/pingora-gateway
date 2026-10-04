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

impl Drop for StartupLock {
    fn drop(&mut self) {
        // Explicitly end the parent's lease even if a clone or a forked pre-exec child still
        // holds this open file description. An unlock error is not an immediate-release proof;
        // ordinary File destruction remains the fallback and may await inherited handle closure.
        let _ = self._file.unlock();
    }
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

#[cfg(test)]
mod descriptor_release_tests {
    use super::*;

    /// A cloned native handle holds the same description until the parent explicitly unlocks.
    #[test]
    fn parent_drop_releases_lease_while_a_duplicate_description_remains_open() {
        let directory = tempfile::tempdir_in(std::env::temp_dir()).unwrap();
        let path = directory.path().join("duplicate-description.lock");
        let owner = StartupLock::acquire_at(&path, Duration::from_secs(1));
        let duplicate = owner
            ._file
            .try_clone()
            .expect("duplicate the owned native description");
        let contender = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let held = contender.try_lock();
        drop(owner);
        let released = contender.try_lock();
        // Close the owned duplicate before assertions, including on the predecessor failure path.
        drop(duplicate);
        assert!(matches!(held, Err(TryLockError::WouldBlock)));
        released.expect("parent drop must release the lease while a duplicate is still open");
        contender.unlock().unwrap();
    }
}
