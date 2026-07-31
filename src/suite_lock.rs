//! An advisory per-suite server lock (`EIMP-1` §S.5, plan Phase C "Suite
//! lockfile"): a second review server — of ANY kind, the long-lived
//! standalone `serve` or a future TUI-owned private one (§S.7a) — started
//! against a suite another server already holds must refuse to start.
//!
//! The lock file's path is derived from the suite's **canonicalized**
//! directory (so two different relative paths naming the same suite collide
//! on the same lock, as they must) and lives under
//! [`crate::journal::journal_dir`]'s scratch/state area — never inside the
//! suite itself. This is exactly `EIMP-1` §S.6's reasoning for the journal
//! ("ephemeral session/process state, not part of the reviewed corpus,
//! should not travel with it or show up in `git status`"), applied to the
//! lock for the same reason.
//!
//! **Stale-vs-live, same discipline as the socket probe.** The lock file's
//! *content* is the live server's own socket path — so a second server
//! probes that recorded socket with `UnixStream::connect`, the exact test
//! [`crate::review_server::serve_uds`] already uses to distinguish a live
//! server from a crashed one's leftover file. A lock file naming a socket
//! nothing is listening on is stale and is replaced, not treated as a
//! permanent wedge.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{EinmoError, Result};

/// The path a suite's advisory server lock lives at, derived from `suite`'s
/// canonicalized directory. Two different (relative, symlinked, `..`-laden)
/// paths naming the same suite resolve to the same lock path.
///
/// # Errors
///
/// Returns an error if `suite` cannot be canonicalized (e.g. it doesn't
/// exist).
pub fn suite_lock_path(suite: &Path) -> Result<PathBuf> {
    let canonical = suite.canonicalize().map_err(|e| EinmoError::io(suite, e))?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_os_str().as_encoded_bytes());
    let digest = hasher.finalize();
    // First 16 bytes (128 bits) is ample collision resistance for a local
    // advisory lock filename -- this is not a security boundary, only a
    // stable, filesystem-safe name derived from the suite's identity.
    Ok(crate::journal::journal_dir().join(format!("suite-{}.lock", hex::encode(&digest[..16]))))
}

/// An acquired advisory suite lock. Held for the server's lifetime;
/// released (the lock file removed) on `Drop`, so a clean shutdown leaves no
/// trace, exactly like [`crate::review_server::serve_uds`]'s own socket
/// cleanup.
#[derive(Debug)]
pub struct SuiteLock {
    path: PathBuf,
}

impl SuiteLock {
    /// Acquire the advisory lock for `suite`, recording `socket_path` (the
    /// new server's own listening socket) as the lock's content so a later
    /// probe can tell whether this server is still alive.
    ///
    /// # Errors
    ///
    /// Returns [`EinmoError::Io`] with [`ErrorKind::AddrInUse`] if a live
    /// server already holds this suite (a `UnixStream::connect` to the
    /// recorded socket succeeds) — never silently steals the lock. Also
    /// errors if `suite` cannot be canonicalized, or if the lock file
    /// itself cannot be written.
    pub fn acquire(suite: &Path, socket_path: &Path) -> Result<Self> {
        let lock_path = suite_lock_path(suite)?;
        if lock_path.exists() {
            if let Ok(existing) = std::fs::read_to_string(&lock_path) {
                let existing_socket = PathBuf::from(existing.trim());
                if !existing_socket.as_os_str().is_empty()
                    && std::os::unix::net::UnixStream::connect(&existing_socket).is_ok()
                {
                    return Err(EinmoError::io(
                        &lock_path,
                        std::io::Error::new(
                            ErrorKind::AddrInUse,
                            format!(
                                "suite {} is already being reviewed by a live server \
                                 (socket {})",
                                suite.display(),
                                existing_socket.display()
                            ),
                        ),
                    ));
                }
            }
            // Stale: nothing is listening on the recorded socket (or it was
            // unreadable/empty). Same "genuinely dead, safe to reclaim"
            // reasoning as serve_uds's own stale-socket handling.
            let _ = std::fs::remove_file(&lock_path);
        }
        if let Some(parent) = lock_path.parent() {
            crate::journal::harden_dir(parent).map_err(|e| EinmoError::io(parent, e))?;
        }
        std::fs::write(&lock_path, socket_path.as_os_str().as_encoded_bytes())
            .map_err(|e| EinmoError::io(&lock_path, e))?;
        Ok(SuiteLock { path: lock_path })
    }

    /// The lock file's own path (for tests, and for a frontend that wants
    /// to point a human at it).
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SuiteLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes every test that touches the process-global
    /// `EINMO_JOURNAL_DIR` (the lock's own scratch base), mirroring
    /// `journal.rs`'s own `ENV_LOCK` discipline.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn scratch_env() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
        let guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        // SAFETY: serialized by `ENV_LOCK` above -- no other test reads or
        // writes EINMO_JOURNAL_DIR while this guard is held.
        unsafe {
            std::env::set_var("EINMO_JOURNAL_DIR", tmp.path());
        }
        (guard, tmp)
    }

    #[test]
    fn suite_lock_path_is_stable_across_relative_and_absolute_forms() {
        let (_guard, _scratch) = scratch_env();
        let suite = tempfile::tempdir().unwrap();
        let direct = suite_lock_path(suite.path()).unwrap();
        let via_dot = suite_lock_path(&suite.path().join(".")).unwrap();
        assert_eq!(direct, via_dot);
    }

    #[test]
    fn suite_lock_path_differs_for_different_suites() {
        let (_guard, _scratch) = scratch_env();
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        assert_ne!(
            suite_lock_path(a.path()).unwrap(),
            suite_lock_path(b.path()).unwrap()
        );
    }

    #[test]
    fn acquire_writes_the_socket_path_and_releases_on_drop() {
        let (_guard, _scratch) = scratch_env();
        let suite = tempfile::tempdir().unwrap();
        let socket = suite.path().join("fake.sock");
        let lock_path;
        {
            let lock = SuiteLock::acquire(suite.path(), &socket).unwrap();
            lock_path = lock.path().to_path_buf();
            assert!(lock_path.exists());
            let contents = std::fs::read_to_string(&lock_path).unwrap();
            assert_eq!(contents, socket.to_string_lossy());
        }
        assert!(!lock_path.exists(), "lock must be released on drop");
    }

    #[test]
    fn acquire_refuses_a_second_lock_while_the_first_socket_is_live() {
        let (_guard, _scratch) = scratch_env();
        let suite = tempfile::tempdir().unwrap();
        let socket_dir = tempfile::tempdir().unwrap();
        let socket_path = socket_dir.path().join("live.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket_path).unwrap();
        // Keep the listener alive (not accepting) -- a pending connect
        // attempt still succeeds against a bound-and-listening socket.
        let _keep_alive = listener;

        let _first = SuiteLock::acquire(suite.path(), &socket_path).unwrap();
        let second = SuiteLock::acquire(suite.path(), &socket_path);
        assert!(second.is_err(), "a live socket must refuse a second lock");
        assert!(matches!(
            second.unwrap_err(),
            EinmoError::Io { source, .. } if source.kind() == ErrorKind::AddrInUse
        ));
    }

    #[test]
    fn acquire_reclaims_a_stale_lock_whose_socket_is_gone() {
        let (_guard, _scratch) = scratch_env();
        let suite = tempfile::tempdir().unwrap();
        let socket_dir = tempfile::tempdir().unwrap();
        let dead_socket = socket_dir.path().join("dead.sock");
        // Never bound -- nothing is listening.
        let stale = SuiteLock::acquire(suite.path(), &dead_socket).unwrap();
        let lock_path = stale.path().to_path_buf();
        std::mem::forget(stale); // simulate a crash: lock file left behind

        let new_socket = socket_dir.path().join("new.sock");
        let fresh = SuiteLock::acquire(suite.path(), &new_socket);
        assert!(
            fresh.is_ok(),
            "a stale lock (dead socket) must be reclaimed, not wedge forever"
        );
        let contents = std::fs::read_to_string(&lock_path).unwrap();
        assert_eq!(contents, new_socket.to_string_lossy());
    }

    #[test]
    fn acquire_errors_on_a_nonexistent_suite() {
        let (_guard, _scratch) = scratch_env();
        let missing = std::path::PathBuf::from("/nonexistent/einmo-suite-lock-test");
        assert!(SuiteLock::acquire(&missing, Path::new("/tmp/x.sock")).is_err());
    }
}
