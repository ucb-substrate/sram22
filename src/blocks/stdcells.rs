//! SKY130 `sky130_fd_sc_hs` standard cells used by the SRAM control logic.
//!
//! The Substrate 2 SKY130 PDK ships schematics for the `sky130_fd_sc_hd` library only, so the
//! high-speed cells used here are read directly from the open PDK's SPICE files.

use std::path::PathBuf;

use arcstr::ArcStr;
use serde::{Deserialize, Serialize};
use sky130::Sky130;
use spice::Spice;
use substrate::block::Block;
use substrate::error::Result;
use substrate::schematic::{CellBuilder, Schematic};
use substrate::types::schematic::IoNodeBundle;
use substrate::types::{InOut, Input, Io, Output, Signal};

/// The `sky130_fd_sc_hs` library name.
pub const HS_LIB: &str = "sky130_fd_sc_hs";

/// Path to the SPICE netlist of a standard cell in the open PDK.
pub fn stdcell_spice_path(lib: &str, name: &str, strength: usize) -> PathBuf {
    PathBuf::from(crate::SKY130_OPEN_PDK_ROOT).join(format!(
        "libraries/{lib}/latest/cells/{name}/{lib}__{name}_{strength}.spice"
    ))
}

/// The power and body IO of a SKY130 standard cell.
#[derive(Debug, Default, Clone, Copy, Io)]
pub struct StdCellPowerIo {
    /// The ground rail.
    pub vgnd: InOut<Signal>,
    /// The power rail.
    pub vpwr: InOut<Signal>,
    /// The pwell body contact.
    pub vnb: InOut<Signal>,
    /// The nwell body contact.
    pub vpb: InOut<Signal>,
}

macro_rules! define_hs_stdcell {
    (
        $(#[$meta:meta])*
        $typ:ident, $io:ident, $name:literal,
        strengths = [$($strength:literal),*],
        ports = [$(($field:ident, $port:literal, $dir:ident)),* $(,)?]
    ) => {
        #[derive(Debug, Default, Clone, Copy, Io)]
        #[doc = concat!("The IO of the `", $name, "` standard cell.")]
        pub struct $io {
            /// The power and body connections.
            pub pwr: StdCellPowerIo,
            $(pub $field: $dir<Signal>,)*
        }

        $(#[$meta])*
        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $typ {
            strength: usize,
        }

        impl $typ {
            #[doc = concat!("Creates a `", $name, "` cell with the given drive strength.")]
            ///
            /// Panics if the strength is not available in the library.
            pub const fn new(strength: usize) -> Self {
                assert!(
                    matches!(strength, $($strength)|*),
                    concat!("unsupported drive strength for ", $name)
                );
                Self { strength }
            }

            /// The drive strength of this cell.
            pub const fn strength(&self) -> usize {
                self.strength
            }
        }

        impl Block for $typ {
            type Io = $io;

            fn name(&self) -> ArcStr {
                arcstr::format!("{}__{}_{}", HS_LIB, $name, self.strength)
            }

            fn io(&self) -> Self::Io {
                Default::default()
            }
        }

        impl Schematic for $typ {
            type Schema = Sky130;
            type NestedData = ();

            fn schematic(
                &self,
                io: &IoNodeBundle<Self>,
                cell: &mut CellBuilder<<Self as Schematic>::Schema>,
            ) -> Result<Self::NestedData> {
                let cell_name = format!("{}__{}_{}", HS_LIB, $name, self.strength);
                let mut scir = Spice::scir_cell_from_file(
                    stdcell_spice_path(HS_LIB, $name, self.strength),
                    &cell_name,
                )
                .convert_schema::<Sky130>()?;
                scir.connect("VGND", io.pwr.vgnd);
                scir.connect("VPWR", io.pwr.vpwr);
                scir.connect("VNB", io.pwr.vnb);
                scir.connect("VPB", io.pwr.vpb);
                $(scir.connect($port, io.$field);)*
                cell.set_scir(scir);
                Ok(())
            }
        }
    };
}

define_hs_stdcell!(
    /// A high-speed inverter.
    HsInv, HsInvIo, "inv",
    strengths = [1, 2, 4, 8, 16],
    ports = [(a, "A", Input), (y, "Y", Output)]
);

define_hs_stdcell!(
    /// A high-speed buffer.
    HsBuf, HsBufIo, "buf",
    strengths = [1, 2, 4, 8, 16],
    ports = [(a, "A", Input), (x, "X", Output)]
);

define_hs_stdcell!(
    /// A high-speed 2-input AND gate.
    HsAnd2, HsAnd2Io, "and2",
    strengths = [1, 2, 4],
    ports = [(a, "A", Input), (b, "B", Input), (x, "X", Output)]
);

define_hs_stdcell!(
    /// A high-speed 2-input NAND gate.
    HsNand2, HsNand2Io, "nand2",
    strengths = [1, 2, 4, 8],
    ports = [(a, "A", Input), (b, "B", Input), (y, "Y", Output)]
);

define_hs_stdcell!(
    /// A high-speed 2-input NOR gate.
    HsNor2, HsNor2Io, "nor2",
    strengths = [1, 2, 4, 8],
    ports = [(a, "A", Input), (b, "B", Input), (y, "Y", Output)]
);

define_hs_stdcell!(
    /// A high-speed 2-input multiplexer.
    HsMux2, HsMux2Io, "mux2",
    strengths = [1, 2, 4],
    ports = [(a0, "A0", Input), (a1, "A1", Input), (s, "S", Input), (x, "X", Output)]
);

define_hs_stdcell!(
    /// A high-speed positive edge triggered flip-flop with inverted reset and
    /// complementary outputs.
    HsDfrbp, HsDfrbpIo, "dfrbp",
    strengths = [1, 2],
    ports = [
        (clk, "CLK", Input),
        (d, "D", Input),
        (reset_b, "RESET_B", Input),
        (q, "Q", Output),
        (q_n, "Q_N", Output),
    ]
);

/// Connects the power and body pins of a standard cell instance to `vdd` and `vss`.
pub fn connect_stdcell_power<T>(
    cell: &mut CellBuilder<Sky130>,
    pwr: T,
    vdd: substrate::types::schematic::Node,
    vss: substrate::types::schematic::Node,
) where
    T: substrate::types::Flatten<substrate::types::schematic::Node>
        + substrate::types::HasBundleKind<BundleKind = StdCellPowerIoKind>,
{
    cell.connect(
        pwr,
        substrate::types::schematic::NodeBundle::<StdCellPowerIo> {
            vgnd: vss,
            vpwr: vdd,
            vnb: vss,
            vpb: vdd,
        },
    );
}

impl crate::schematic::FromParams for HsInv {
    type Params = usize;
    fn from_params(p: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self::new(*p))
    }
}
crate::impl_sky130_build!(HsInv);

impl crate::schematic::FromParams for HsBuf {
    type Params = usize;
    fn from_params(p: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self::new(*p))
    }
}
crate::impl_sky130_build!(HsBuf);

impl crate::schematic::FromParams for HsAnd2 {
    type Params = usize;
    fn from_params(p: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self::new(*p))
    }
}
crate::impl_sky130_build!(HsAnd2);

impl crate::schematic::FromParams for HsNand2 {
    type Params = usize;
    fn from_params(p: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self::new(*p))
    }
}
crate::impl_sky130_build!(HsNand2);

impl crate::schematic::FromParams for HsNor2 {
    type Params = usize;
    fn from_params(p: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self::new(*p))
    }
}
crate::impl_sky130_build!(HsNor2);

impl crate::schematic::FromParams for HsMux2 {
    type Params = usize;
    fn from_params(p: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self::new(*p))
    }
}
crate::impl_sky130_build!(HsMux2);

impl crate::schematic::FromParams for HsDfrbp {
    type Params = usize;
    fn from_params(p: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self::new(*p))
    }
}
crate::impl_sky130_build!(HsDfrbp);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, substrate::block::Block)]
#[substrate(io = "HdDfrtpIo")]
pub struct HdDfrtp;
#[derive(Debug, Clone, Default, Io)]
pub struct HdDfrtpIo {
    pub clk: Input<Signal>,
    pub d: Input<Signal>,
    pub reset_b: Input<Signal>,
    pub q: Output<Signal>,
    pub vgnd: InOut<Signal>,
    pub vpwr: InOut<Signal>,
    pub vnb: InOut<Signal>,
    pub vpb: InOut<Signal>,
}
impl Schematic for HdDfrtp {
    type Schema = Sky130;
    type NestedData = ();
    fn schematic(&self, io: &IoNodeBundle<Self>, cell: &mut CellBuilder<Sky130>) -> Result<()> {
        let mut scir = Spice::scir_cell_from_file(
            stdcell_spice_path("sky130_fd_sc_hd", "dfrtp", 2),
            "sky130_fd_sc_hd__dfrtp_2",
        )
        .convert_schema::<Sky130>()?;
        scir.connect("CLK", io.clk);
        scir.connect("D", io.d);
        scir.connect("RESET_B", io.reset_b);
        scir.connect("Q", io.q);
        scir.connect("VGND", io.vgnd);
        scir.connect("VPWR", io.vpwr);
        scir.connect("VNB", io.vnb);
        scir.connect("VPB", io.vpb);
        cell.set_scir(scir);
        Ok(())
    }
}
impl crate::schematic::FromParams for HdDfrtp {
    type Params = crate::schematic::NoParams;
    fn from_params(_: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
crate::impl_sky130_build!(HdDfrtp);
