//! Embedded input extraction and process-local storage.
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Result;
use lazy_static::lazy_static;
use tempfile::TempDir;

const FILES: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/assets.rs"));

lazy_static! {
    // A private process-owned directory avoids shared-cache races and stale assets.
    static ref INPUTS: Mutex<Option<TempDir>> = Mutex::new(None);
}

fn unpack() -> Result<TempDir> {
    let directory = tempfile::Builder::new()
        .prefix("sram22-assets-")
        .tempdir()?;
    for (name, bytes) in FILES {
        let path = directory.path().join(name);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, bytes)?;
    }
    Ok(directory)
}

pub(crate) fn path(relative: &str) -> PathBuf {
    let mut inputs = INPUTS.lock().unwrap();
    inputs
        .get_or_insert_with(|| unpack().expect("failed to unpack embedded SRAM22 inputs"))
        .path()
        .join(relative)
}

/// Release process-local files after all generator contexts and tasks have ended.
pub fn cleanup() {
    INPUTS.lock().unwrap().take();
}

pub(crate) fn standard_cell_root() -> PathBuf {
    path("tech/sky130")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_inputs_are_extracted() {
        assert!(path("templates/sram_1rw_wmask.v").is_file());
        assert!(path("tech/sky130/gds/sram_sp_cell.gds").is_file());
        assert!(standard_cell_root()
            .join("gds/sky130_fd_sc_hs__inv_2.gds")
            .is_file());
        assert!(standard_cell_root()
            .join("spice/sky130_fd_sc_hs__inv_2.spice")
            .is_file());
        assert!(!standard_cell_root().join("libraries").exists());
    }
}
