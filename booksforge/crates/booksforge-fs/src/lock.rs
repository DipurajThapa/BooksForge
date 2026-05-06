//! Advisory process lock for a bundle directory.
//!
//! The lock file (`.booksforge.lock`) contains the PID of the holding process
//! on a single line.  Before refusing to open, we check whether that PID is
//! still alive — if the process is dead the lock is stale and is evicted.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("bundle is locked by another process (PID {pid})")]
    AlreadyLocked { pid: u32 },

    #[error("I/O error acquiring lock at {path}: {source}")]
    Io { path: String, source: std::io::Error },
}

/// RAII guard that holds a lock file for the duration of its lifetime.
/// Drop to release the lock.
pub struct BundleLock {
    path: PathBuf,
}

impl BundleLock {
    /// Attempt to acquire the lock.
    ///
    /// 1. If the lock file does not exist: create it with our PID and succeed.
    /// 2. If the lock file exists: read the PID.
    ///    - If the process is alive: return `LockError::AlreadyLocked`.
    ///    - If the process is dead (stale lock): delete the file and retry (once).
    pub fn acquire(lock_path: PathBuf) -> Result<Self, LockError> {
        for attempt in 0..2 {
            match try_create_lock(&lock_path) {
                Ok(()) => return Ok(Self { path: lock_path }),
                Err(LockError::AlreadyLocked { pid }) => {
                    if attempt == 0 && !pid_is_alive(pid) {
                        // Stale lock — remove and retry once.
                        let _ = std::fs::remove_file(&lock_path);
                        continue;
                    }
                    return Err(LockError::AlreadyLocked { pid });
                }
                Err(e) => return Err(e),
            }
        }
        // Should be unreachable after one retry.
        Err(LockError::AlreadyLocked { pid: 0 })
    }
}

impl Drop for BundleLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn try_create_lock(path: &PathBuf) -> Result<(), LockError> {
    use std::fs::OpenOptions;
    use std::io::{Read, Write};

    if path.exists() {
        // Read the PID from the existing lock file.
        let mut content = String::new();
        std::fs::File::open(path)
            .and_then(|mut f| f.read_to_string(&mut content))
            .map_err(|e| LockError::Io {
                path: path.display().to_string(),
                source: e,
            })?;
        let pid: u32 = content.trim().parse().unwrap_or(0);
        return Err(LockError::AlreadyLocked { pid });
    }

    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                LockError::AlreadyLocked { pid: 0 }
            } else {
                LockError::Io { path: path.display().to_string(), source: e }
            }
        })?;

    writeln!(f, "{}", std::process::id()).map_err(|e| LockError::Io {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(())
}

/// Check whether a process with `pid` is currently running.
///
/// On Unix: `kill(pid, 0)` returns `Ok(())` if the process exists.
/// On Windows: `OpenProcess` returns a non-null handle if alive.
fn pid_is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // Safety: signal 0 never kills — it just checks process existence.
        let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
        result == 0
    }

    #[cfg(windows)]
    {
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        use windows::Win32::Foundation::CloseHandle;
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
            if let Ok(h) = handle {
                let _ = CloseHandle(h);
                true
            } else {
                false
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        false // conservative on unknown platforms
    }
}
