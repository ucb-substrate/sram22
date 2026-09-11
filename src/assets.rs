//! Embedded input extraction and process-local storage.
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Result;
use flate2::read::GzDecoder;
use lazy_static::lazy_static;
use tempfile::TempDir;

const FILES: &[(&str, &[u8])] = include!(concat!(env!("OUT_DIR"), "/assets.rs"));
const SKY130_ARCHIVE: &[u8] = include_bytes!("../tech/sky130/pdk/sky130.tar.gz");

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
    tar::Archive::new(GzDecoder::new(SKY130_ARCHIVE)).unpack(directory.path().join("pdk"))?;
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
    path("pdk")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_inputs_are_extracted() {
        assert!(path("templates/sram_1rw_wmask.v").is_file());
        assert!(path("tech/sky130/gds/sram_sp_cell.gds").is_file());
        assert!(standard_cell_root()
            .join("libraries/sky130_fd_sc_hs/latest/cells/inv/sky130_fd_sc_hs__inv_2.gds")
            .is_file());
        assert!(standard_cell_root()
            .join("libraries/sky130_fd_sc_hs/latest/cells/inv/sky130_fd_sc_hs__inv_2.spice")
            .is_file());
        assert!(!standard_cell_root().join("libraries/sky130_fd_pr").exists());
    }
}
