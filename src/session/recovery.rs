use std::path::{Path, PathBuf};

use super::session_path;

pub fn recovery_dir() -> Option<PathBuf> {
    session_path().and_then(|p| p.parent().map(|d| d.join("recovery")))
}

pub fn recovery_file_name(path: &Path) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in path.to_string_lossy().as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}.bak")
}
