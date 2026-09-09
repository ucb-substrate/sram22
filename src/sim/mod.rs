//! Simulation support shared by all testbenches.
//!
//! SRAM22 selects a simulator at compile time: [ngspice](ngspice::Ngspice) with the
//! open-source SKY130 PDK by default, or Spectre with the `spectre` feature. The BWRC `commercial`
//! feature selects Spectre and the NDA PDK. This module hides that choice behind a small set of type aliases and
//! helpers so that testbenches can be written once.

use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
pub mod blocks;
pub mod pdk;
pub mod waveform;

#[cfg(not(feature = "spectre"))]
mod backend {
    use rust_decimal::Decimal;

    /// The circuit simulator in use.
    pub type Simulator = ngspice::Ngspice;
    /// Per-simulation options for [`Simulator`].
    pub type Options = ngspice::Options;
    /// A transient analysis for [`Simulator`].
    pub type Tran = ngspice::tran::Tran;
    /// A transient output waveform produced by [`Simulator`].
    pub type OutputWaveform = ngspice::tran::OutputWaveform;
    /// The SKY130 PDK flavor matching [`Simulator`].
    pub type PdkSchema = super::pdk::OpenPdkSchema;

    /// Creates a transient analysis running until `stop` seconds with the given suggested
    /// time step.
    pub fn tran(stop: Decimal, step: Decimal) -> Tran {
        Tran {
            stop,
            step,
            start: None,
        }
    }
}

#[cfg(feature = "spectre")]
mod backend {
    use rust_decimal::Decimal;

    /// The circuit simulator in use.
    pub type Simulator = spectre::Spectre;
    /// Per-simulation options for [`Simulator`].
    pub type Options = spectre::Options;
    /// A transient analysis for [`Simulator`].
    pub type Tran = spectre::analysis::tran::Tran;
    /// A transient output waveform produced by [`Simulator`].
    pub type OutputWaveform = spectre::analysis::tran::OutputWaveform;
    /// The SKY130 PDK flavor matching [`Simulator`].
    #[cfg(feature = "commercial")]
    pub type PdkSchema = sky130::Sky130SrcNdaSchema;
    #[cfg(not(feature = "commercial"))]
    pub type PdkSchema = super::pdk::OpenPdkSchema;

    /// Creates a transient analysis running until `stop` seconds.
    ///
    /// Spectre chooses its own time steps; `step` is ignored.
    pub fn tran(stop: Decimal, _step: Decimal) -> Tran {
        Tran {
            stop,
            start: None,
            errpreset: Some(spectre::ErrPreset::Conservative),
            ..Default::default()
        }
    }
}

pub use backend::*;

/// Converts an `f64` to a [`Decimal`], rounding to 15 significant digits.
///
/// Panics if `x` is not finite.
pub fn dec(x: f64) -> Decimal {
    let d = Decimal::from_f64(x).unwrap_or_else(|| panic!("cannot convert {x} to a decimal"));
    d.round_sf(15).unwrap_or(d).normalize()
}

pub mod run;
pub use run::{run, run_with_corner};

#[cfg(not(feature = "spectre"))]
pub mod ac;

#[cfg(test)]
mod tests;
