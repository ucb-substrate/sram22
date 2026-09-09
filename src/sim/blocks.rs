//! Simulator-independent sources and passives for testbenches.
//!
//! These blocks are written in the [`Spice`] schema as blackbox SPICE elements, which every
//! SPICE-like simulator supported by Substrate (ngspice, Spectre in SPICE mode) understands.

use rust_decimal::Decimal;
use spice::{BlackboxContents, BlackboxElement, Primitive, Spice};
use substrate::block::Block;
use substrate::error::Result;
use substrate::schematic::{CellBuilder, PrimitiveBinding, Schematic};
use substrate::simulation::waveform::{TimeWaveform, Waveform};
use substrate::types::schematic::IoNodeBundle;
use substrate::types::TwoTerminalIo;

pub use spice::Resistor;

/// Builds a two-terminal blackbox SPICE element.
///
/// The element is written as `{prefix}{instance name} {p} {n} {tail}`.
fn two_terminal_blackbox(prefix: &str, tail: String) -> Primitive {
    let contents = BlackboxContents {
        elems: vec![
            BlackboxElement::RawString(prefix.into()),
            BlackboxElement::InstanceName,
            BlackboxElement::RawString(" ".into()),
            BlackboxElement::Port("p".into()),
            BlackboxElement::RawString(" ".into()),
            BlackboxElement::Port("n".into()),
            BlackboxElement::RawString(format!(" {tail}").into()),
        ],
    };
    Primitive::BlackboxInstance { contents }
}

fn bind_two_terminal(
    io: &substrate::types::schematic::NodeBundle<TwoTerminalIo>,
    cell: &mut CellBuilder<Spice>,
    prim: Primitive,
) {
    let mut prim = PrimitiveBinding::new(prim);
    prim.connect("p", io.p);
    prim.connect("n", io.n);
    cell.set_primitive(prim);
}

/// A DC voltage source.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, Block)]
#[substrate(io = "TwoTerminalIo")]
pub struct Vdc {
    /// The source voltage (V).
    pub value: Decimal,
}

impl Vdc {
    /// Creates a new DC voltage source.
    pub fn new(value: Decimal) -> Self {
        Self { value }
    }
}

impl Schematic for Vdc {
    type Schema = Spice;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        bind_two_terminal(
            io,
            cell,
            two_terminal_blackbox("V", format!("DC {}", self.value)),
        );
        Ok(())
    }
}

/// A pulse voltage source.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, Block)]
#[substrate(io = "TwoTerminalIo")]
pub struct Vpulse {
    /// The initial value (V).
    pub val0: Decimal,
    /// The pulsed value (V).
    pub val1: Decimal,
    /// The delay before the first pulse (s).
    pub delay: Decimal,
    /// The rise time (s).
    pub rise: Decimal,
    /// The fall time (s).
    pub fall: Decimal,
    /// The pulse width (s).
    pub width: Decimal,
    /// The pulse period (s).
    pub period: Decimal,
}

impl Schematic for Vpulse {
    type Schema = Spice;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        bind_two_terminal(
            io,
            cell,
            two_terminal_blackbox(
                "V",
                format!(
                    "PULSE({} {} {} {} {} {} {})",
                    self.val0, self.val1, self.delay, self.rise, self.fall, self.width, self.period
                ),
            ),
        );
        Ok(())
    }
}

/// A piecewise linear voltage source.
#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, Block)]
#[substrate(io = "TwoTerminalIo")]
pub struct Vpwl {
    /// The (time, voltage) waveform.
    pub waveform: Waveform<Decimal>,
}

impl Vpwl {
    /// Creates a new PWL voltage source.
    pub fn new(waveform: Waveform<Decimal>) -> Self {
        Self { waveform }
    }
}

impl Schematic for Vpwl {
    type Schema = Spice;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        use std::fmt::Write;
        let mut pwl = String::from("PWL(");
        for (i, pt) in self.waveform.values().enumerate() {
            if i != 0 {
                pwl.push(' ');
            }
            write!(&mut pwl, "{} {}", pt.t(), pt.x()).unwrap();
        }
        pwl.push(')');
        bind_two_terminal(io, cell, two_terminal_blackbox("V", pwl));
        Ok(())
    }
}

/// A DC current source.
///
/// Positive current flows into the `p` terminal and out of the `n` terminal.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, Block)]
#[substrate(io = "TwoTerminalIo")]
pub struct Idc {
    /// The source current (A).
    pub value: Decimal,
}

impl Idc {
    /// Creates a new DC current source.
    pub fn new(value: Decimal) -> Self {
        Self { value }
    }
}

impl Schematic for Idc {
    type Schema = Spice;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        bind_two_terminal(
            io,
            cell,
            two_terminal_blackbox("I", format!("DC {}", self.value)),
        );
        Ok(())
    }
}

/// An AC small-signal current source with zero DC current.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, Block)]
#[substrate(io = "TwoTerminalIo")]
pub struct Iac {
    /// The AC magnitude (A).
    pub mag: Decimal,
}

impl Iac {
    /// Creates a new AC current source.
    pub fn new(mag: Decimal) -> Self {
        Self { mag }
    }
}

impl Schematic for Iac {
    type Schema = Spice;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        bind_two_terminal(
            io,
            cell,
            two_terminal_blackbox("I", format!("DC 0 AC {}", self.mag)),
        );
        Ok(())
    }
}

/// An ideal capacitor.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, Block)]
#[substrate(io = "TwoTerminalIo")]
pub struct Capacitor {
    /// The capacitance (F).
    pub value: Decimal,
}

impl Capacitor {
    /// Creates a new capacitor.
    pub fn new(value: Decimal) -> Self {
        Self { value }
    }
}

impl Schematic for Capacitor {
    type Schema = Spice;
    type NestedData = ();

    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<<Self as Schematic>::Schema>,
    ) -> Result<Self::NestedData> {
        let mut prim = PrimitiveBinding::new(Primitive::Cap2 { value: self.value });
        prim.connect("1", io.p);
        prim.connect("2", io.n);
        cell.set_primitive(prim);
        Ok(())
    }
}

macro_rules! source_constructor {
    ($t:ty) => {
        impl crate::schematic::FromParams for $t {
            type Params = Decimal;
            fn from_params(p: &Decimal) -> anyhow::Result<Self> {
                Ok(Self::new(*p))
            }
        }
        impl crate::schematic::BuildIn<crate::sim::Simulator> for $t {
            fn build_in(self) -> crate::schematic::CircuitInstance<crate::sim::Simulator> {
                crate::schematic::native_instance(substrate::schematic::ConvertSchema::<
                    _,
                    crate::sim::Simulator,
                >::new(self))
            }
        }
    };
}
source_constructor!(Vdc);
source_constructor!(Idc);
source_constructor!(Iac);
source_constructor!(Capacitor);
source_constructor!(Resistor);
impl crate::schematic::FromParams for Vpulse {
    type Params = Self;
    fn from_params(p: &Self) -> anyhow::Result<Self> {
        Ok(*p)
    }
}
impl crate::schematic::FromParams for Vpwl {
    type Params = std::sync::Arc<Waveform<f64>>;
    fn from_params(p: &Self::Params) -> anyhow::Result<Self> {
        use crate::sim::waveform::DigitalWaveform;
        Ok(Self::new(p.to_decimal()))
    }
}
macro_rules! source_build {
    ($t:ty) => {
        impl crate::schematic::BuildIn<crate::sim::Simulator> for $t {
            fn build_in(self) -> crate::schematic::CircuitInstance<crate::sim::Simulator> {
                crate::schematic::native_instance(substrate::schematic::ConvertSchema::<
                    _,
                    crate::sim::Simulator,
                >::new(self))
            }
        }
    };
}
source_build!(Vpulse);
source_build!(Vpwl);
