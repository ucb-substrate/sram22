use crate::blocks::gate::{Inv, PrimitiveGateParams};
use crate::sim::blocks::Vdc;
use crate::sim::blocks::Vpulse;
use crate::sim::run::SimulationTestbench as Testbench;
use crate::sim::run::TranAnalysis;
use rust_decimal::Decimal;

use super::*;

pub struct TdcTb {
    params: TdcTbParams,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct TdcTbParams {
    pub inner: TdcParams,
    pub vdd: f64,
    /// Difference between input waveform rising edges.
    pub delta_t: f64,
    /// Rise time.
    pub tr: f64,
    /// Simulation end time.
    pub t_stop: f64,
}

impl TdcTb {
    // Float bit patterns give block identity a reflexive Eq and matching Hash, including NaNs.
    fn cache_key(&self) -> impl std::hash::Hash + Eq + '_ {
        let p = &self.params;
        (
            [
                p.vdd.to_bits(),
                p.delta_t.to_bits(),
                p.tr.to_bits(),
                p.t_stop.to_bits(),
            ],
            &p.inner,
        )
    }
}

impl std::hash::Hash for TdcTb {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        std::hash::Hash::hash(&self.cache_key(), h)
    }
}
impl PartialEq for TdcTb {
    fn eq(&self, other: &Self) -> bool {
        self.cache_key() == other.cache_key()
    }
}
impl Eq for TdcTb {}
impl crate::schematic::FromParams for TdcTb {
    type Params = TdcTbParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self { params: *params })
    }
}
impl substrate::block::Block for TdcTb {
    type Io = substrate::types::TestbenchIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("tdc_testbench")
    }
    fn io(&self) -> Self::Io {
        Default::default()
    }
}
impl substrate::schematic::Schematic for TdcTb {
    type Schema = crate::sim::Simulator;
    type NestedData = ();
    fn schematic(
        &self,
        io: &substrate::types::schematic::IoNodeBundle<Self>,
        cell: &mut substrate::schematic::CellBuilder<Self::Schema>,
    ) -> substrate::error::Result<()> {
        let mut ctx = crate::schematic::CircuitBuilder::new(
            &<Self as substrate::block::Block>::io(self),
            io,
            cell,
        );
        self.build_schematic(&mut ctx)
            .map_err(|e| substrate::error::Error::Anyhow(std::sync::Arc::new(e)))
    }
}
impl TdcTb {
    fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder<crate::sim::Simulator>,
    ) -> anyhow::Result<()> {
        let vss = ctx.port("vss", crate::schematic::Direction::InOut);
        let [vdd, a, b, b0, reset_b] = ctx.signals(["vdd", "a", "b", "b0", "reset_b"]);
        let dout = ctx.bus("dout", self.params.inner.bits_out());

        let vmax = crate::sim::dec(self.params.vdd);
        ctx.instantiate::<Vdc>(&vmax)?
            .with_connections([("p", vdd), ("n", vss)])
            .named("Vvdd")
            .add_to(ctx);

        let t0 = 100e-12;
        let ta = crate::sim::dec(t0);
        let tb = crate::sim::dec(self.params.delta_t + t0);
        let tr = crate::sim::dec(self.params.tr);
        let treset = crate::sim::dec(t0 / 2.0);

        ctx.instantiate::<Vpulse>(&Vpulse {
            val0: Decimal::ZERO,
            val1: vmax,
            delay: treset,
            rise: tr,
            fall: tr,
            width: crate::sim::dec((1000) as f64 * 1.),
            period: crate::sim::dec((2000) as f64 * 1.),
        })?
        .with_connections([("p", reset_b), ("n", vss)])
        .named("Vreset")
        .add_to(ctx);

        ctx.instantiate::<Vpulse>(&Vpulse {
            val0: Decimal::ZERO,
            val1: vmax,
            delay: ta,
            rise: tr,
            fall: tr,
            width: crate::sim::dec((1000) as f64 * 1.),
            period: crate::sim::dec((2000) as f64 * 1.),
        })?
        .with_connections([("p", a), ("n", vss)])
        .named("Va")
        .add_to(ctx);

        ctx.instantiate::<Vpulse>(&Vpulse {
            val0: vmax,
            val1: Decimal::ZERO,
            delay: tb,
            rise: tr,
            fall: tr,
            width: crate::sim::dec((1000) as f64 * 1.),
            period: crate::sim::dec((2000) as f64 * 1.),
        })?
        .with_connections([("p", b0), ("n", vss)])
        .named("Vb")
        .add_to(ctx);

        let inv_params = PrimitiveGateParams {
            nwidth: 3_000,
            pwidth: 12_000,
            length: 150,
        };

        ctx.instantiate::<Inv>(&inv_params)?
            .with_connections([("vdd", vdd), ("vss", vss), ("a", b0), ("y", b)])
            .named("Xbbuf")
            .add_to(ctx);

        ctx.instantiate::<Tdc>(&self.params.inner)?
            .with_connections([
                ("vdd", vdd),
                ("vss", vss),
                ("reset_b", reset_b),
                ("a", a),
                ("b", b),
                ("dout", dout),
            ])
            .named("Xdut")
            .add_to(ctx);

        Ok(())
    }
}

impl Testbench for TdcTb {
    type Output = ();
    fn setup(&self, ctx: &mut crate::sim::run::SimulationPlan) -> anyhow::Result<()> {
        let tran = TranAnalysis::builder()
            .start(0.0)
            .stop(self.params.t_stop)
            .step(self.params.delta_t / 10.0)
            .build()
            .unwrap();
        ctx.add_analysis(tran);
        ctx.save(crate::sim::run::Save::All);
        Ok(())
    }

    fn measure(&self, _ctx: &crate::sim::run::SimulationResults) -> anyhow::Result<Self::Output> {
        Ok(())
    }
}
