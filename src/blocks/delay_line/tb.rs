use crate::sim::blocks::Vdc;
use crate::sim::blocks::Vpulse;
use crate::sim::run::SimulationTestbench as Testbench;
use crate::sim::run::TranAnalysis;
use rust_decimal::Decimal;

use super::*;

pub struct DelayLineTb {
    params: DelayLineTbParams,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub enum DelayLineKind {
    Naive(NaiveDelayLineParams),
    TristateInv(TristateInvDelayLineParams),
}

impl DelayLineKind {
    fn stages(self) -> usize {
        match self {
            DelayLineKind::Naive(params) => params.stages,
            DelayLineKind::TristateInv(params) => params.stages,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelayLineTbParams {
    pub inner: DelayLineKind,
    pub vdd: f64,
    /// Rise time.
    pub tr: f64,
    /// Clock frequency.
    pub f: f64,
    /// Time at each delay setting.
    pub ctl_period: f64,
    /// Simulation end time.
    ///
    /// Defaults to `inner.stages * ctl_period`.
    pub t_stop: Option<f64>,
}

impl DelayLineTb {
    // Float bit patterns give block identity a reflexive Eq and matching Hash, including NaNs.
    fn cache_key(&self) -> impl std::hash::Hash + Eq + '_ {
        let p = &self.params;
        (
            [
                p.vdd.to_bits(),
                p.tr.to_bits(),
                p.f.to_bits(),
                p.ctl_period.to_bits(),
            ],
            &p.inner,
            p.t_stop.map(f64::to_bits),
        )
    }
}

impl std::hash::Hash for DelayLineTb {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        std::hash::Hash::hash(&self.cache_key(), h)
    }
}
impl PartialEq for DelayLineTb {
    fn eq(&self, other: &Self) -> bool {
        self.cache_key() == other.cache_key()
    }
}
impl Eq for DelayLineTb {}
impl crate::schematic::FromParams for DelayLineTb {
    type Params = DelayLineTbParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self { params: *params })
    }
}
impl substrate::block::Block for DelayLineTb {
    type Io = substrate::types::TestbenchIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("delay_line_testbench")
    }
    fn io(&self) -> Self::Io {
        Default::default()
    }
}
impl substrate::schematic::Schematic for DelayLineTb {
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
impl DelayLineTb {
    fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder<crate::sim::Simulator>,
    ) -> anyhow::Result<()> {
        let stages = self.params.inner.stages();

        let vss = ctx.port("vss", crate::schematic::Direction::InOut);
        let [vdd, clk_in, clk_out] = ctx.signals(["vdd", "clk_in", "clk_out"]);
        let ctl = ctx.bus("ctl", stages);
        let ctl_b = ctx.bus("ctl_b", stages);

        let vmax = crate::sim::dec(self.params.vdd);
        ctx.instantiate::<Vdc>(&vmax)?
            .with_connections([("p", vdd), ("n", vss)])
            .named("Vvdd")
            .add_to(ctx);

        let clk_period = crate::sim::dec(1. / self.params.f);
        let half_clk_period = crate::sim::dec(1. / 2. / self.params.f);
        let tr = crate::sim::dec(self.params.tr);
        let ctl_period = crate::sim::dec(self.params.ctl_period);
        let anti_ctl_period = crate::sim::dec((stages - 1) as f64 * self.params.ctl_period);
        let all_ctl_period = crate::sim::dec(stages as f64 * self.params.ctl_period);

        ctx.instantiate::<Vpulse>(&Vpulse {
            val0: Decimal::ZERO,
            val1: vmax,
            delay: Decimal::ZERO,
            rise: tr,
            fall: tr,
            width: half_clk_period,
            period: clk_period,
        })?
        .with_connections([("p", clk_in), ("n", vss)])
        .named("Vclk")
        .add_to(ctx);

        for i in 0..stages {
            for j in 0..2 {
                ctx.instantiate::<Vpulse>(&Vpulse {
                    val0: Decimal::ZERO,
                    val1: vmax,
                    delay: crate::sim::dec(
                        (i as f64 + j as f64 - stages as f64) * self.params.ctl_period,
                    ),
                    rise: tr,
                    fall: tr,
                    width: if j == 0 { ctl_period } else { anti_ctl_period },
                    period: all_ctl_period,
                })?
                .with_connections([("p", if j == 0 { ctl } else { ctl_b }.index(i)), ("n", vss)])
                .named("Va")
                .add_to(ctx);
            }
        }

        match self.params.inner {
            DelayLineKind::Naive(params) => {
                ctx.instantiate::<NaiveDelayLine>(&params)?
                    .with_connections([
                        ("vdd", vdd),
                        ("vss", vss),
                        ("clk_in", clk_in),
                        ("clk_out", clk_out),
                        ("ctl", ctl),
                        ("ctl_b", ctl_b),
                    ])
                    .named("Xdut")
                    .add_to(ctx);
            }
            DelayLineKind::TristateInv(params) => {
                ctx.instantiate::<TristateInvDelayLine>(&params)?
                    .with_connections([
                        ("vdd", vdd),
                        ("vss", vss),
                        ("clk_in", clk_in),
                        ("clk_out", clk_out),
                        ("ctl", ctl),
                        ("ctl_b", ctl_b),
                    ])
                    .named("Xdut")
                    .add_to(ctx);
            }
        }

        Ok(())
    }
}

impl Testbench for DelayLineTb {
    type Output = ();
    fn setup(&self, ctx: &mut crate::sim::run::SimulationPlan) -> anyhow::Result<()> {
        let tran = TranAnalysis::builder()
            .start(0.0)
            .stop(
                self.params
                    .t_stop
                    .unwrap_or(self.params.inner.stages() as f64 * self.params.ctl_period),
            )
            .step(1. / 10. / self.params.f)
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
