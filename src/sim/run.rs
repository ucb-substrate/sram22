//! Testbench execution and portable saved simulation data.
use super::{dec, Options, Simulator};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use substrate::context::Context;
#[cfg(feature = "spectre")]
use substrate::simulation::SimController;
use substrate::simulation::Testbench;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealSignal {
    pub values: Arc<Vec<f64>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplexSignal {
    pub real: Vec<f64>,
    pub imag: Vec<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranData {
    pub time: RealSignal,
    pub data: HashMap<String, RealSignal>,
}
impl TranData {
    pub fn waveform(&self, name: &str) -> Option<super::OutputWaveform> {
        let sig = self
            .data
            .get(name)
            .or_else(|| self.data.get(&name.to_ascii_lowercase()))?;
        Some(super::OutputWaveform {
            t: self.time.values.clone(),
            x: sig.values.clone(),
        })
    }
}
#[derive(Debug, Clone)]
pub struct AcData {
    pub freq: RealSignal,
    pub data: HashMap<String, ComplexSignal>,
}
pub enum AnalysisData {
    Tran(TranData),
    Ac(AcData),
}
impl AnalysisData {
    pub fn tran(&self) -> &TranData {
        match self {
            Self::Tran(d) => d,
            _ => panic!("expected transient results"),
        }
    }
    pub fn ac(&self) -> &AcData {
        match self {
            Self::Ac(d) => d,
            _ => panic!("expected AC results"),
        }
    }
}
pub struct SimulationResults {
    pub data: Vec<AnalysisData>,
}
#[derive(Debug, Clone, Builder)]
pub struct TranAnalysis {
    pub stop: f64,
    pub step: f64,
    #[builder(default)]
    pub start: f64,
}
impl TranAnalysis {
    pub fn builder() -> TranAnalysisBuilder {
        TranAnalysisBuilder::default()
    }
}
#[derive(Debug, Clone, Copy)]
pub enum SweepMode {
    Lin,
    Dec,
    Oct,
}
#[derive(Debug, Clone, Builder)]
pub struct AcAnalysis {
    pub fstart: f64,
    pub fstop: f64,
    pub points: usize,
    pub sweep: SweepMode,
}
impl AcAnalysis {
    pub fn builder() -> AcAnalysisBuilder {
        AcAnalysisBuilder::default()
    }
}
#[derive(Debug, Clone)]
pub enum Analysis {
    Tran(TranAnalysis),
    Ac(AcAnalysis),
}
impl From<TranAnalysis> for Analysis {
    fn from(a: TranAnalysis) -> Self {
        Self::Tran(a)
    }
}
#[derive(Debug, Clone)]
pub enum Save {
    All,
    Signals(HashSet<String>),
}
#[derive(Default)]
pub struct SimulationPlan {
    analyses: Vec<Analysis>,
    includes: Vec<PathBuf>,
    saves: Vec<Save>,
    ics: Vec<(String, rust_decimal::Decimal)>,
}
impl SimulationPlan {
    pub fn add_analysis(&mut self, a: impl Into<Analysis>) -> &mut Self {
        self.analyses.push(a.into());
        self
    }
    pub fn include(&mut self, p: impl AsRef<Path>) -> &mut Self {
        self.includes.push(p.as_ref().into());
        self
    }
    pub fn save(&mut self, s: Save) -> &mut Self {
        self.saves.push(s);
        self
    }
    pub fn set_ic(&mut self, p: impl Into<String>, v: rust_decimal::Decimal) {
        self.ics.push((p.into(), v));
    }
}
/// An executable Substrate 2 testbench with application-specific measurements.
pub trait SimulationTestbench: Testbench<Simulator> + crate::schematic::FromParams {
    type Output;
    fn setup(&self, plan: &mut SimulationPlan) -> anyhow::Result<()>;
    fn measure(&self, results: &SimulationResults) -> anyhow::Result<Self::Output>;
}

fn signal_name(name: &str) -> String {
    let name = name
        .strip_prefix("v(")
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(name);
    name.to_ascii_lowercase()
}
/// Executes a testbench through the Substrate 2 simulation controller.
pub fn run<T: SimulationTestbench>(
    ctx: &Context,
    params: &T::Params,
    work_dir: impl AsRef<Path>,
) -> anyhow::Result<T::Output> {
    run_with_corner::<T>(ctx, params, work_dir, sky130::corner::Sky130Corner::Tt)
}
pub fn run_with_corner<T: SimulationTestbench>(
    ctx: &Context,
    params: &T::Params,
    work_dir: impl AsRef<Path>,
    corner: sky130::corner::Sky130Corner,
) -> anyhow::Result<T::Output> {
    std::fs::create_dir_all(work_dir.as_ref())?;
    let work_dir = work_dir.as_ref().canonicalize()?;
    let sim = ctx.get_sim_controller::<Simulator, T>(T::from_params(params)?, &work_dir)?;
    let tb = sim.tb.block();
    let mut plan = SimulationPlan::default();
    tb.setup(&mut plan)?;
    // Includes are supplied relative to the caller, while simulators run in work_dir.
    plan.includes = plan
        .includes
        .iter()
        .map(|path| path.canonicalize())
        .collect::<std::io::Result<_>>()?;
    let mut opts = Options::default();
    sim.set_option(corner, &mut opts);
    for p in &plan.includes {
        opts.include(p);
    }
    let save_all = plan.saves.is_empty() || plan.saves.iter().any(|s| matches!(s, Save::All));
    if save_all {
        #[cfg(not(feature = "spectre"))]
        opts.save_tran_voltage("all");
        #[cfg(feature = "spectre")]
        opts.save(spectre::SaveOption::All);
    }
    let mut signals: HashSet<String> = plan
        .saves
        .iter()
        .filter_map(|s| match s {
            Save::Signals(signals) => Some(signals.iter().cloned()),
            Save::All => None,
        })
        .flatten()
        .collect();
    signals.extend(plan.ics.iter().map(|(name, _)| name.clone()));
    #[cfg(not(feature = "spectre"))]
    if signals.iter().any(|s| s.contains('*')) {
        let lib = ctx.export_scir(T::from_params(params)?)?;
        signals = expand_signal_patterns(&lib.scir, signals);
    }
    let extracted = crate::pex::ExtractedNodes::read(&plan.includes)?;
    let mut aliases = HashMap::new();
    for signal in signals {
        let resolved = extracted.resolve(&signal)?;
        if resolved != signal {
            aliases.insert(signal_name(&signal), signal_name(&resolved));
        }
        let signal = resolved;
        #[cfg(not(feature = "spectre"))]
        opts.save_tran_voltage(format!("v({})", signal.to_ascii_lowercase()));
        #[cfg(feature = "spectre")]
        opts.save_tran_voltage(signal);
    }
    if !plan.ics.is_empty() {
        let ics: Vec<_> = plan
            .ics
            .iter()
            .map(|(name, value)| Ok((extracted.resolve(name)?, *value)))
            .collect::<anyhow::Result<_>>()?;
        #[cfg(feature = "spectre")]
        for (path, value) in ics {
            use substrate::simulation::options::ic::{InitialCondition, Voltage};
            sim.set_option(
                InitialCondition {
                    path,
                    value: Voltage(value),
                },
                &mut opts,
            );
        }
        #[cfg(not(feature = "spectre"))]
        {
            std::fs::create_dir_all(work_dir.as_path())?;
            let path = work_dir.as_path().join("sram22_initial_conditions.spice");
            let contents = ics
                .iter()
                .map(|(n, v)| format!(".ic v({n})={v}\n"))
                .collect::<String>();
            std::fs::write(&path, contents)?;
            opts.include(path);
        }
    }
    let mut data = Vec::new();
    for analysis in plan.analyses {
        match analysis {
            Analysis::Tran(a) => {
                let mut tran = super::tran(dec(a.stop), dec(a.step));
                tran.start = Some(dec(a.start));
                let output = sim.simulate_default(opts.clone(), tran)?;
                let mut d = TranData {
                    time: RealSignal {
                        values: output.time,
                    },
                    data: output
                        .raw_values
                        .into_iter()
                        .map(|(n, v)| (signal_name(&n), RealSignal { values: v }))
                        .collect(),
                };
                for (alias, actual) in &aliases {
                    if let Some(value) = d.data.get(actual).cloned() {
                        d.data.insert(alias.clone(), value);
                    }
                }
                std::fs::create_dir_all(work_dir.as_path())?;
                let file = std::fs::File::create(work_dir.as_path().join("transient.json"))?;
                let mut writer = std::io::BufWriter::new(file);
                serde_json::to_writer(&mut writer, &d)?;
                std::io::Write::flush(&mut writer)?;
                data.push(AnalysisData::Tran(d));
            }
            Analysis::Ac(a) => {
                #[cfg(feature = "spectre")]
                let result = run_ac(&sim, opts.clone(), a)?;
                #[cfg(not(feature = "spectre"))]
                let result = {
                    let sim = ctx.get_sim_controller::<super::ac::NgspiceAc, T>(
                        T::from_params(params)?,
                        work_dir.as_path(),
                    )?;
                    sim.simulate_default(
                        super::ac::AcOptions {
                            includes: plan.includes.clone(),
                            corner: Some(corner),
                        },
                        a,
                    )?
                };
                data.push(AnalysisData::Ac(result));
            }
        }
    }
    tb.measure(&SimulationResults { data })
}
#[cfg(feature = "spectre")]
fn run_ac<T: Testbench<Simulator>>(
    sim: &SimController<Simulator, T>,
    opts: Options,
    a: AcAnalysis,
) -> anyhow::Result<AcData> {
    let sweep = match a.sweep {
        SweepMode::Lin => spectre::analysis::Sweep::Linear(a.points),
        SweepMode::Dec => spectre::analysis::Sweep::Decade(a.points),
        SweepMode::Oct => spectre::analysis::Sweep::Logarithmic(
            ((a.fstop / a.fstart).log2() * a.points as f64).ceil() as usize + 1,
        ),
    };
    let out = sim.simulate_default(
        opts,
        spectre::analysis::ac::Ac {
            start: dec(a.fstart),
            stop: dec(a.fstop),
            sweep,
        },
    )?;
    Ok(AcData {
        freq: RealSignal { values: out.freq },
        data: out
            .raw_values
            .into_iter()
            .map(|(n, v)| {
                (
                    signal_name(&n),
                    ComplexSignal {
                        real: v.iter().map(|v| v.re).collect(),
                        imag: v.iter().map(|v| v.im).collect(),
                    },
                )
            })
            .collect(),
    })
}

impl RealSignal {
    pub fn get(&self, i: usize) -> Option<f64> {
        self.values.get(i).copied()
    }
    pub fn where_at_least(&self, v: f64) -> Option<usize> {
        self.values.iter().position(|x| *x >= v)
    }
    pub fn idx_before_sorted(&self, x: f64) -> Option<usize> {
        self.values.partition_point(|v| *v <= x).checked_sub(1)
    }
}

/// ngspice does not expand hierarchical wildcards in .save statements.
#[cfg(not(feature = "spectre"))]
fn expand_signal_patterns(
    lib: &scir::Library<ngspice::Ngspice>,
    signals: HashSet<String>,
) -> HashSet<String> {
    let (patterns, signals): (Vec<String>, Vec<String>) =
        signals.into_iter().partition(|s| s.contains('*'));
    let mut signals: HashSet<String> = signals.into_iter().collect();
    let patterns: Vec<_> = patterns
        .iter()
        .map(|p| {
            regex::Regex::new(&format!("(?i)^{}$", regex::escape(p).replace(r"\*", ".*"))).unwrap()
        })
        .collect();
    fn walk(
        lib: &scir::Library<ngspice::Ngspice>,
        cell: scir::CellId,
        prefix: &str,
        patterns: &[regex::Regex],
        out: &mut HashSet<String>,
    ) {
        let cell = lib.cell(cell);
        for (_, signal) in cell.signals() {
            for i in 0..signal.width.unwrap_or(1) {
                let name = if signal.width.is_some() {
                    format!("{prefix}{}[{i}]", signal.name)
                } else {
                    format!("{prefix}{}", signal.name)
                };
                if patterns.iter().any(|p| p.is_match(&name)) {
                    out.insert(name);
                }
            }
        }
        for (_, inst) in cell.instances() {
            if let scir::ChildId::Cell(child) = inst.child() {
                walk(
                    lib,
                    child,
                    &format!("{prefix}X{}.", inst.name()),
                    patterns,
                    out,
                );
            }
        }
    }
    if let Some(top) = lib.top_cell() {
        walk(lib, top, "", &patterns, &mut signals);
    }
    signals
}
