//! SKY130 dimensions, custom cell locations, and open PDK selection.
use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};

pub const SKY130_DOMAIN: &str = "sky130";
pub const SRAM_SP_CELL: &str = "sram_sp_cell";
pub const SRAM_SP_COLEND: &str = "sky130_fd_bd_sram__sram_sp_colend";
pub const SRAM_SP_CELL_REPLICA: &str = "sram_sp_cell_replica";
pub const OPENRAM_DFF: &str = "openram_dff";
pub const SRAM_CONTROL_SIMPLE: &str = "sramgen_control_simple";
pub const SRAM_CONTROL_REPLICA_V1: &str = "sramgen_control_replica_v1";
pub const SRAM_CONTROL_BUFBUF_16: &str = "control_logic_bufbuf_16";
pub const SRAM_SP_SENSE_AMP: &str = "sramgen_sp_sense_amp";
pub const CONTROL_LOGIC_INV: &str = "control_logic_inv";

pub const BITCELL_HEIGHT: isize = 1580;
pub const BITCELL_WIDTH: isize = 1200;
pub const TAPCELL_WIDTH: isize = 1300;
pub const COLUMN_WIDTH: isize = BITCELL_WIDTH + TAPCELL_WIDTH;

#[inline]
pub fn custom_gds_dir() -> PathBuf {
    crate::assets::path("tech/sky130/gds")
}

#[inline]
pub fn custom_spice_dir() -> PathBuf {
    crate::assets::path("tech/sky130/spice")
}

const OPEN_MODEL_LIBRARY: &str = "libraries/sky130_fd_pr/latest/models/sky130.lib.spice";

fn resolve_open_pdk_root(override_path: Option<OsString>) -> Result<PathBuf> {
    let root = match override_path {
        Some(value) if value.is_empty() => bail!("SKY130_OPEN_PDK_ROOT must not be empty"),
        Some(value) => PathBuf::from(value),
        None => crate::assets::bundled_sky130_root(),
    };
    let root = root
        .canonicalize()
        .with_context(|| format!("invalid SKY130 PDK root: {}", root.display()))?;
    for required in [
        "libraries/sky130_fd_sc_hs/latest/cells/inv/sky130_fd_sc_hs__inv_2.gds",
        "libraries/sky130_fd_sc_hs/latest/cells/inv/sky130_fd_sc_hs__inv_2.spice",
        OPEN_MODEL_LIBRARY,
    ] {
        if !root.join(required).is_file() {
            bail!("SKY130 PDK {} is missing {required}", root.display());
        }
    }
    Ok(root)
}

/// Select the open PDK, honoring the runtime SKY130_OPEN_PDK_ROOT override.
pub fn open_pdk_root() -> Result<PathBuf> {
    resolve_open_pdk_root(std::env::var_os("SKY130_OPEN_PDK_ROOT"))
}

/// The device-model library used for portable SPICE exports.
pub fn open_model_library() -> Result<PathBuf> {
    Ok(open_pdk_root()?.join(OPEN_MODEL_LIBRARY))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_pdk_and_explicit_override() {
        let bundled = resolve_open_pdk_root(None).unwrap();
        assert_eq!(
            resolve_open_pdk_root(Some(bundled.clone().into())).unwrap(),
            bundled
        );
        assert!(resolve_open_pdk_root(Some(OsString::new())).is_err());
        let missing = tempfile::tempdir().unwrap();
        assert!(resolve_open_pdk_root(Some(missing.path().into())).is_err());
    }
}
