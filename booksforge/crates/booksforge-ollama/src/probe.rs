//! Ollama binary detection and process launch helpers.
//!
//! These are *infrastructure* helpers: they touch the filesystem and spawn
//! processes.  Keep them free of any domain logic.

use std::path::PathBuf;

/// Result of probing the local environment for Ollama.
#[derive(Debug, Clone)]
pub struct ProbeResult {
    /// Whether Ollama's HTTP API is reachable on `127.0.0.1:11434`.
    pub api_reachable: bool,
    /// Server version string, if reachable.
    pub version: Option<String>,
    /// Whether an Ollama binary was found on disk (even if the server is not running).
    pub binary_found: bool,
    /// Path to the binary, if found.
    pub binary_path: Option<PathBuf>,
}

/// Candidate paths to check for the Ollama binary, in order of preference.
fn binary_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![
        PathBuf::from("/usr/local/bin/ollama"),
        PathBuf::from("/usr/bin/ollama"),
    ];

    // User-local install.
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(PathBuf::from(&home).join(".local/bin/ollama"));
        candidates.push(PathBuf::from(&home).join("bin/ollama"));
    }

    // macOS: bundled app places a binary at a known location.
    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from(
            "/Applications/Ollama.app/Contents/Resources/ollama",
        ));
        // Homebrew.
        candidates.push(PathBuf::from("/opt/homebrew/bin/ollama"));
        candidates.push(PathBuf::from("/usr/local/homebrew/bin/ollama"));
    }

    // Windows.
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(&local).join("Programs").join("Ollama").join("ollama.exe"),
            );
        }
        if let Ok(prog) = std::env::var("PROGRAMFILES") {
            candidates.push(PathBuf::from(&prog).join("Ollama").join("ollama.exe"));
        }
    }

    candidates
}

/// Find the Ollama binary on disk.  Returns the first path that exists.
pub fn find_binary() -> Option<PathBuf> {
    // Also check PATH via `which`-style lookup.
    if let Ok(path) = which_ollama() {
        return Some(path);
    }
    binary_candidates().into_iter().find(|p| p.exists())
}

fn which_ollama() -> Result<PathBuf, ()> {
    let name = if cfg!(target_os = "windows") { "ollama.exe" } else { "ollama" };
    let path_var = std::env::var("PATH").map_err(|_| ())?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(())
}

/// Attempt to launch the Ollama application.
///
/// On macOS this opens the `Ollama.app` bundle (which starts the menu-bar
/// process and the HTTP server).  On Windows it launches `ollama.exe` in the
/// background.  On Linux it runs `ollama serve` in the background.
///
/// Returns `Ok(())` if the spawn succeeded — does NOT wait for Ollama to
/// become ready (the caller should poll `version()` afterwards).
pub fn launch_ollama() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let app_bundle = std::path::Path::new("/Applications/Ollama.app");
        if app_bundle.exists() {
            std::process::Command::new("open")
                .arg("-a")
                .arg("Ollama")
                .spawn()
                .map_err(|e| format!("failed to open Ollama.app: {e}"))?;
            return Ok(());
        }
        // Fall through to binary launch if the .app bundle is not present.
    }

    let binary = find_binary()
        .ok_or_else(|| "Ollama binary not found on this system".to_owned())?;

    std::process::Command::new(&binary)
        .arg("serve")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to start ollama serve: {e}"))?;

    Ok(())
}

/// Detect available system RAM in gigabytes using `/proc/meminfo` (Linux),
/// `sysctl` (macOS), or the Windows memory API.
///
/// Returns `None` if detection fails — callers should show all models.
pub fn available_ram_gb() -> Option<u32> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let out = Command::new("sysctl")
            .arg("-n")
            .arg("hw.memsize")
            .output()
            .ok()?;
        let bytes: u64 = String::from_utf8(out.stdout).ok()?.trim().parse().ok()?;
        Some((bytes / 1_073_741_824) as u32)
    }

    #[cfg(target_os = "linux")]
    {
        let content = std::fs::read_to_string("/proc/meminfo").ok()?;
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let kb: u64 = line
                    .split_whitespace()
                    .nth(1)?
                    .parse()
                    .ok()?;
                return Some((kb / 1_048_576) as u32);
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    {
        // MEMORYSTATUSEX.ullTotalPhys via GlobalMemoryStatusEx.
        use std::mem;
        #[repr(C)]
        struct MemoryStatusEx {
            dw_length:                  u32,
            dw_memory_load:             u32,
            ull_total_phys:             u64,
            ull_avail_phys:             u64,
            ull_total_page_file:        u64,
            ull_avail_page_file:        u64,
            ull_total_virtual:          u64,
            ull_avail_virtual:          u64,
            ull_avail_extended_virtual: u64,
        }
        extern "system" {
            fn GlobalMemoryStatusEx(lp_buffer: *mut MemoryStatusEx) -> i32;
        }
        // SAFETY: `MemoryStatusEx` is a `#[repr(C)]` plain-old-data struct;
        // every field is an integer with a defined zero representation, so
        // `mem::zeroed()` produces a valid (if uninitialised-meaningful) value.
        // We immediately overwrite `dw_length` below before passing the struct
        // to the kernel, which is the only field the API requires us to set.
        let mut status: MemoryStatusEx = unsafe { mem::zeroed() };
        status.dw_length = mem::size_of::<MemoryStatusEx>() as u32;
        // SAFETY: `&mut status` is a valid, exclusive pointer to the
        // `MemoryStatusEx` struct above.  `GlobalMemoryStatusEx` is documented
        // to write only to the buffer we pass and only up to `dw_length`
        // bytes (which we set to `size_of::<MemoryStatusEx>()`), so the call
        // cannot overrun.  No Rust borrow invariant is violated because
        // `status` is not borrowed elsewhere while the FFI call holds the
        // pointer.  The function takes care of its own thread-safety.
        let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
        if ok != 0 {
            Some((status.ull_total_phys / 1_073_741_824) as u32)
        } else {
            None
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_candidates_is_non_empty() {
        assert!(!binary_candidates().is_empty());
    }

    #[test]
    fn available_ram_gb_returns_plausible_value_or_none() {
        // We can't assert exact values across all CI machines, but if detection
        // succeeds the value should be within a reasonable range.
        if let Some(gb) = available_ram_gb() {
            assert!(gb >= 1, "RAM should be at least 1 GB");
            assert!(gb <= 4096, "RAM above 4 TB is implausible");
        }
    }
}
