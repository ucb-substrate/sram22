use crate::schematic::{CircuitBuilder, FromParams, NoParams};
use crate::sim::blocks::{Capacitor, Iac, Resistor, Vpulse};
use crate::sim::run::{
    AcAnalysis, Analysis, SimulationPlan, SimulationResults, SimulationTestbench, SweepMode,
    TranAnalysis,
};
use crate::sim::{dec, Simulator};
use substrate::block::Block;
use substrate::schematic::{CellBuilder, Schematic};
use substrate::types::schematic::IoNodeBundle;
use substrate::types::TestbenchIo;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Block)]
#[substrate(io = "TestbenchIo")]
struct RcTestbench;

impl FromParams for RcTestbench {
    type Params = NoParams;
    fn from_params(_: &NoParams) -> anyhow::Result<Self> {
        Ok(Self)
    }
}

impl Schematic for RcTestbench {
    type Schema = Simulator;
    type NestedData = ();
    fn schematic(
        &self,
        io: &IoNodeBundle<Self>,
        cell: &mut CellBuilder<Simulator>,
    ) -> substrate::error::Result<()> {
        let mut c = CircuitBuilder::new(&self.io(), io, cell);
        let gnd = c.port("vss", crate::schematic::Direction::InOut);
        let [input, output, ac] = c.signals(["input", "output", "ac"]);
        c.instantiate::<Vpulse>(&Vpulse {
            val0: dec(0.),
            val1: dec(1.),
            delay: dec(1e-6),
            rise: dec(1e-9),
            fall: dec(1e-9),
            width: dec(10e-6),
            period: dec(20e-6),
        })
        .unwrap()
        .with_connections([("p", input), ("n", gnd)])
        .named("stim")
        .add_to(&mut c);
        for (name, p, n) in [("r_tran", input, output), ("r_ac", ac, gnd)] {
            c.instantiate::<Resistor>(&dec(1000.))
                .unwrap()
                .with_connections([("p", p), ("n", n)])
                .named(name)
                .add_to(&mut c);
        }
        for (name, p) in [("c_tran", output), ("c_ac", ac)] {
            c.instantiate::<Capacitor>(&dec(1e-9))
                .unwrap()
                .with_connections([("p", p), ("n", gnd)])
                .named(name)
                .add_to(&mut c);
        }
        c.instantiate::<Iac>(&dec(1.))
            .unwrap()
            .with_connections([("p", gnd), ("n", ac)])
            .named("ac_stim")
            .add_to(&mut c);
        Ok(())
    }
}

impl SimulationTestbench for RcTestbench {
    type Output = ();
    fn setup(&self, plan: &mut SimulationPlan) -> anyhow::Result<()> {
        plan.set_ic("output", dec(0.25));
        plan.save(crate::sim::run::Save::Signals(
            ["input*".into(), "output".into(), "ac".into()]
                .into_iter()
                .collect(),
        ));
        plan.add_analysis(TranAnalysis {
            stop: 5e-6,
            start: 0.,
            step: 1e-8,
        });
        plan.add_analysis(Analysis::Ac(AcAnalysis {
            fstart: 1e3,
            fstop: 1e6,
            points: 4,
            sweep: SweepMode::Dec,
        }));
        Ok(())
    }
    fn measure(&self, result: &SimulationResults) -> anyhow::Result<()> {
        let tran = result.data[0].tran();
        let i = tran.time.where_at_least(2e-6).unwrap();
        assert!(tran.data.contains_key("input"));
        assert!((tran.data["output"].values[0] - 0.25).abs() < 1e-6);
        let expected = 1. - (-(tran.time.values[i] - 1.0005e-6) / 1e-6).exp()
            + 0.25 * (-tran.time.values[i] / 1e-6).exp();
        assert!((tran.data["output"].values[i] - expected).abs() < 0.005);
        let ac = result.data[1].ac();
        for (i, f) in ac.freq.values.iter().enumerate() {
            let wc = 2. * std::f64::consts::PI * f * 1e-6;
            let re = 1000. / (1. + wc * wc);
            let im = -wc * re;
            assert!((ac.data["ac"].real[i] - re).abs() < 1e-4);
            assert!((ac.data["ac"].imag[i] - im).abs() < 1e-4);
        }
        Ok(())
    }
}

#[test]
#[ignore = "requires the selected simulator and SKY130 models"]
fn transient_and_ac() {
    let dir = tempfile::tempdir().unwrap();
    crate::sim::run::<RcTestbench>(&crate::setup_ctx(), &NoParams, dir.path()).unwrap();
    assert!(dir.path().join("transient.json").is_file());
}
