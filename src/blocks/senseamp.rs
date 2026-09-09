use crate::sim::waveform::{DigitalTrace as Waveform, DigitalWaveform};
use std::sync::Arc;

use crate::bits::is_logical_low;
use crate::schematic::NoParams;
use crate::sim::blocks::Capacitor;
use crate::sim::blocks::Vdc;
use crate::sim::blocks::Vpwl;
use crate::sim::run::SimulationTestbench as Testbench;
use crate::sim::run::TranAnalysis;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use substrate::simulation::waveform::TimeWaveform;

use super::macros::SenseAmp;

pub struct OffsetTb {
    params: OffsetTbParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Offset {
    value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffsetTbParams {
    vnom: f64,
    vdd: f64,
    vincr: f64,
    period: f64,
    tslew: f64,
    n_incr: usize,
    cout: Decimal,
}

impl OffsetTb {
    fn waveforms(&self) -> (Waveform, Waveform) {
        let vdd = self.params.vdd;
        let ts = self.params.tslew;
        let period = self.params.period;
        let vnom = self.params.vnom;
        let vincr = self.params.vincr;
        let mut clk = Waveform::with_initial_value(0.0);
        let mut vin = Waveform::with_initial_value(vnom);

        for i in 0..self.params.n_incr + 1 {
            let t = i as f64 * period;
            let t_negedge = t + period / 2.0;
            let t_posedge = t + period;
            clk.push_high(t_negedge, vdd, ts);
            clk.push_low(t_posedge, vdd, ts);
            vin.push(t_negedge, vin.last_x().unwrap());

            let value = vnom - i as f64 * vincr;
            vin.push(t_negedge + ts, value);
        }

        (vin, clk)
    }
}

impl OffsetTb {
    // Float bit patterns give block identity a reflexive Eq and matching Hash, including NaNs.
    fn cache_key(&self) -> impl std::hash::Hash + Eq + '_ {
        let p = &self.params;
        (
            [
                p.vnom.to_bits(),
                p.vdd.to_bits(),
                p.vincr.to_bits(),
                p.period.to_bits(),
                p.tslew.to_bits(),
            ],
            &p.n_incr,
            &p.cout,
        )
    }
}

impl std::hash::Hash for OffsetTb {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        std::hash::Hash::hash(&self.cache_key(), h)
    }
}
impl PartialEq for OffsetTb {
    fn eq(&self, other: &Self) -> bool {
        self.cache_key() == other.cache_key()
    }
}
impl Eq for OffsetTb {}
impl crate::schematic::FromParams for OffsetTb {
    type Params = OffsetTbParams;
    fn from_params(params: &Self::Params) -> anyhow::Result<Self> {
        Ok(Self {
            params: params.clone(),
        })
    }
}
impl substrate::block::Block for OffsetTb {
    type Io = substrate::types::TestbenchIo;
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("sense_amp_offset_tb")
    }
    fn io(&self) -> Self::Io {
        Default::default()
    }
}
impl substrate::schematic::Schematic for OffsetTb {
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
impl OffsetTb {
    fn build_schematic(
        &self,
        ctx: &mut crate::schematic::CircuitBuilder<crate::sim::Simulator>,
    ) -> anyhow::Result<()> {
        let vss = ctx.port("vss", crate::schematic::Direction::InOut);
        let [clk, inn, inp, outn, outp, vdd] =
            ctx.signals(["clk", "inn", "inp", "outn", "outp", "vdd"]);

        let mut coutp = ctx.instantiate::<Capacitor>(&self.params.cout)?;
        coutp.connect_all([("p", outp), ("n", vss)]);
        coutp.set_name("coutp");
        ctx.add_instance(coutp);

        let mut coutn = ctx.instantiate::<Capacitor>(&self.params.cout)?;
        coutn.connect_all([("p", outn), ("n", vss)]);
        coutn.set_name("coutn");
        ctx.add_instance(coutn);

        let mut vnom = ctx.instantiate::<Vdc>(&crate::sim::dec(self.params.vnom))?;
        vnom.connect_all([("p", inn), ("n", vss)]);
        vnom.set_name("vnom");
        ctx.add_instance(vnom);

        let vvdd = ctx
            .instantiate::<Vdc>(&crate::sim::dec(self.params.vdd))?
            .with_connections([("p", vdd), ("n", vss)])
            .named("vvdd");
        ctx.add_instance(vvdd);

        let (vin, vclk) = self.waveforms();
        let vclk = ctx
            .instantiate::<Vpwl>(&Arc::new(vclk))?
            .with_connections([("p", clk), ("n", vss)])
            .named("vclk");
        ctx.add_instance(vclk);

        let vvin = ctx
            .instantiate::<Vpwl>(&Arc::new(vin))?
            .with_connections([("p", inp), ("n", vss)])
            .named("vvin");
        ctx.add_instance(vvin);

        let mut sa = ctx.instantiate::<SenseAmp>(&NoParams)?;
        sa.connect_all([
            ("clk", clk),
            ("inn", inn),
            ("inp", inp),
            ("outn", outn),
            ("outp", outp),
            ("VDD", vdd),
            ("VSS", vss),
        ]);
        sa.set_name("dut");
        ctx.add_instance(sa);

        Ok(())
    }
}

impl Testbench for OffsetTb {
    type Output = Offset;

    fn setup(&self, ctx: &mut crate::sim::run::SimulationPlan) -> anyhow::Result<()> {
        ctx.add_analysis(
            TranAnalysis::builder()
                .start(0.0)
                .stop(self.params.period * (self.params.n_incr + 1) as f64)
                .step(self.params.period / 40.0)
                .build()
                .unwrap(),
        )
        .save(crate::sim::run::Save::All);
        Ok(())
    }

    fn measure(&self, ctx: &crate::sim::run::SimulationResults) -> anyhow::Result<Self::Output> {
        let data = &ctx.data[0].tran();
        let vout = &data.data["outp"];
        let vinp = &data.data["inp"];
        let t = &data.time;

        let period = self.params.period;

        let mut idx_thresh = None;
        for i in 0..self.params.n_incr + 1 {
            let t_i = i as f64 * period;
            let t_negedge = t_i + period / 2.0;

            let idx = t
                .where_at_least(t_negedge - self.params.period / 10.0)
                .unwrap();
            let vout = vout.values[idx];
            if is_logical_low(vout, self.params.vdd) {
                idx_thresh = Some(idx);
                break;
            }
        }

        let idx_thresh = idx_thresh.unwrap();
        let ofs = self.params.vnom - vinp.values[idx_thresh];
        assert!(ofs >= 0.0, "offset {} expected to be larger than 0", ofs);
        Ok(Offset { value: ofs })
    }
}

#[cfg(test)]
mod tests {

    use crate::setup_ctx;
    use crate::tests::test_work_dir;

    use super::*;

    #[test]
    #[ignore = "slow"]
    fn test_sa_offset_tb() {
        let ctx = setup_ctx();
        let work_dir = test_work_dir("test_sa_offset_tb");
        let params = OffsetTbParams {
            vnom: 1.7,
            vdd: 1.8,
            vincr: 0.1e-3,
            period: 4e-9,
            tslew: 10e-12,
            n_incr: 1_000,
            cout: crate::sim::dec((2) as f64 * 1e-15),
        };
        let offset = crate::sim::run::<OffsetTb>(&ctx, &params, &work_dir)
            .expect("failed to run simulation");
        println!("SA offset = {:?}", offset);
    }
    use crate::schematic::{CircuitBuilder, FromParams, NoParams};
    use crate::sim::blocks::Vpulse;
    use crate::sim::run::{SimulationPlan, SimulationResults, SimulationTestbench, TranAnalysis};
    use crate::sim::{dec, Simulator};
    use substrate::block::Block;
    use substrate::schematic::{CellBuilder, Schematic};
    use substrate::types::schematic::IoNodeBundle;
    use substrate::types::TestbenchIo;

    #[derive(Clone, Copy, Hash, PartialEq, Eq, Block)]
    #[substrate(io = "TestbenchIo")]
    struct SenseAmpTestbench;
    impl FromParams for SenseAmpTestbench {
        type Params = NoParams;
        fn from_params(_: &NoParams) -> anyhow::Result<Self> {
            Ok(Self)
        }
    }
    impl Schematic for SenseAmpTestbench {
        type Schema = Simulator;
        type NestedData = ();
        fn schematic(
            &self,
            io: &IoNodeBundle<Self>,
            cell: &mut CellBuilder<Simulator>,
        ) -> substrate::error::Result<()> {
            use crate::sim::blocks::Vdc;
            let mut c = CircuitBuilder::new(&self.io(), io, cell);
            let gnd = c.port("vss", crate::schematic::Direction::InOut);
            let [vdd, inp, inn, outp, outn, clk] =
                c.signals(["vdd", "inp", "inn", "outp", "outn", "clk"]);
            for (name, node, value) in
                [("supply", vdd, 1.8), ("inp", inp, 0.95), ("inn", inn, 0.85)]
            {
                c.instantiate::<Vdc>(&dec(value))
                    .unwrap()
                    .with_connections([("p", node), ("n", gnd)])
                    .named(name)
                    .add_to(&mut c);
            }
            c.instantiate::<Vpulse>(&Vpulse {
                val0: dec(0.),
                val1: dec(1.8),
                delay: dec(1e-9),
                rise: dec(20e-12),
                fall: dec(20e-12),
                width: dec(5e-9),
                period: dec(10e-9),
            })
            .unwrap()
            .with_connections([("p", clk), ("n", gnd)])
            .named("clock")
            .add_to(&mut c);
            c.instantiate::<crate::blocks::macros::SenseAmp>(&NoParams)
                .unwrap()
                .with_connections([
                    ("vdd", vdd),
                    ("vss", gnd),
                    ("clk", clk),
                    ("inp", inp),
                    ("inn", inn),
                    ("outp", outp),
                    ("outn", outn),
                ])
                .named("dut")
                .add_to(&mut c);
            Ok(())
        }
    }
    impl SimulationTestbench for SenseAmpTestbench {
        type Output = ();
        fn setup(&self, plan: &mut SimulationPlan) -> anyhow::Result<()> {
            plan.add_analysis(TranAnalysis {
                stop: 4e-9,
                start: 0.,
                step: 5e-12,
            });
            Ok(())
        }
        fn measure(&self, results: &SimulationResults) -> anyhow::Result<()> {
            let data = results.data[0].tran();
            let p = *data.data["outp"].values.last().unwrap();
            let n = *data.data["outn"].values.last().unwrap();
            assert!(
                p > 1.6 && n < 0.2,
                "sense amp did not resolve input: outp={p}, outn={n}"
            );
            Ok(())
        }
    }
    #[test]
    #[ignore = "requires the selected simulator and SKY130 models"]
    fn resolves_differential_input() {
        let dir = tempfile::tempdir().unwrap();
        crate::sim::run::<SenseAmpTestbench>(&crate::setup_ctx(), &NoParams, dir.path()).unwrap();
    }
}
