//! One-time move from the working name "dpimngr" (0.1.x) to DPIMech, run by `install`.
//! Profiles and downloaded engines are kept by moving the old data directory; the old
//! service, its binary and its firewall/nftables leftovers are removed by the OS modules.

use dpimech_core::paths::{DataDir, default_service_data_dir};

pub const LEGACY: &str = "dpimngr";

/// Renames `<parent>/dpimngr` to the new default data directory when only the old one exists.
/// A custom `--data-dir` is the user's choice and left alone.
pub fn move_data_dir(data: &DataDir) {
    if data.root != default_service_data_dir() {
        return;
    }
    let old = data.root.with_file_name(LEGACY);
    if !old.is_dir() || data.root.exists() {
        return;
    }
    match std::fs::rename(&old, &data.root) {
        Ok(()) => println!(
            "moved profiles and engines from {} to {}",
            old.display(),
            data.root.display()
        ),
        Err(e) => println!(
            "could not move {} to {}: {e} (profiles start empty; the old folder was kept)",
            old.display(),
            data.root.display()
        ),
    }
}
