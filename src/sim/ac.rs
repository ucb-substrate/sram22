//! AC analysis for the Substrate 2 ngspice schema.
//!
//! The upstream ngspice plugin currently exposes transient analysis only. This simulator
//! uses the same SCIR netlister, executor and raw-file parser to run small-signal analyses.
use super::run::{AcAnalysis, AcData, ComplexSignal, RealSignal, SweepMode};
use spice::netlist::{Include, NetlistKind, NetlistOptions, NetlisterInstance, RenameGround};
use std::io::Write;
use std::path::PathBuf;
use substrate::context::Installation;
use substrate::simulation::{Analysis, SimulationContext, Simulator, SupportedBy};

#[derive(Debug, Default)]
pub struct NgspiceAc;
impl Installation for NgspiceAc {}
#[derive(Debug, Default, Clone)]
pub struct AcOptions {
    pub includes: Vec<PathBuf>,
    pub corner: Option<sky130::corner::Sky130Corner>,
}
impl Analysis for AcAnalysis {
    type Output = AcData;
}
impl SupportedBy<NgspiceAc> for AcAnalysis {
    fn into_input(self, inputs: &mut Vec<AcAnalysis>) {
        inputs.push(self);
    }
    fn from_output(outputs: &mut impl Iterator<Item = AcData>) -> AcData {
        outputs.next().expect("missing AC results")
    }
}
impl Simulator for NgspiceAc {
    type Schema = ngspice::Ngspice;
    type Input = AcAnalysis;
    type Options = AcOptions;
    type Output = AcData;
    type Error = anyhow::Error;
    fn simulate_inputs(
        &self,
        ctx: &SimulationContext<Self>,
        options: AcOptions,
        input: Vec<AcAnalysis>,
    ) -> anyhow::Result<Vec<AcData>> {
        let mut includes: Vec<_> = options.includes.into_iter().map(Include::new).collect();
        if let Some(corner) = options.corner {
            includes.push(
                Include::new(
                    PathBuf::from(crate::SKY130_OPEN_PDK_ROOT)
                        .join("libraries/sky130_fd_pr/latest/models/sky130.lib.spice"),
                )
                .section(corner.name()),
            );
        }
        includes.extend(ctx.lib.scir.primitives().filter_map(|(_, p)| {
            if let ngspice::Primitive::Spice(spice::Primitive::RawInstanceWithInclude {
                netlist,
                ..
            }) = p
            {
                Some(Include::new(netlist))
            } else {
                None
            }
        }));
        includes.sort();
        includes.dedup();
        let mut results = Vec::with_capacity(input.len());
        for (i, a) in input.into_iter().enumerate() {
            anyhow::ensure!(
                a.fstart > 0. && a.fstop > a.fstart && a.points > 0,
                "invalid AC sweep"
            );
            let dir = ctx.work_dir.join(format!("ac_{i}"));
            std::fs::create_dir_all(&dir)?;
            let dir = dir.canonicalize()?;
            let path = dir.join("netlist.spice");
            let raw = dir.join("data.raw");
            let mut netlist = Vec::new();
            NetlisterInstance::new(
                &ngspice::Ngspice::default(),
                &ctx.lib.scir,
                &mut netlist,
                NetlistOptions::new(
                    NetlistKind::Testbench(RenameGround::Yes("0".into())),
                    &includes,
                ),
            )
            .export()?;
            let sweep = match a.sweep {
                SweepMode::Lin => "lin",
                SweepMode::Dec => "dec",
                SweepMode::Oct => "oct",
            };
            writeln!(
                netlist,
                "\n.save all\n.ac {sweep} {} {} {}\n.end",
                a.points, a.fstart, a.fstop
            )?;
            std::fs::write(&path, &netlist)?;
            let mut command = std::process::Command::new("ngspice");
            command
                .args(["-b", "-r"])
                .arg(&raw)
                .arg("-o")
                .arg(dir.join("ngspice.log"))
                .arg(&path)
                .current_dir(&dir);
            ctx.ctx
                .executor
                .execute(command, substrate::execute::ExecOpts::default())?;
            let bytes = std::fs::read(&raw)?;
            let parsed = nutlex::parse(
                &bytes,
                nutlex::Options {
                    endianness: if cfg!(target_endian = "little") {
                        nutlex::ByteOrder::LittleEndian
                    } else {
                        nutlex::ByteOrder::BigEndian
                    },
                },
            )?;
            let analysis = parsed
                .analyses
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("ngspice produced no AC analysis"))?;
            let nutlex::parser::Data::Complex(values) = analysis.data else {
                anyhow::bail!("expected complex AC results")
            };
            let mut data = std::collections::HashMap::new();
            let mut freq = None;
            for var in analysis.variables {
                let v = &values[var.idx];
                if var.name == "frequency" {
                    freq = Some(RealSignal {
                        values: v.real.clone().into(),
                    });
                } else {
                    let name = var
                        .name
                        .strip_prefix("v(")
                        .and_then(|s| s.strip_suffix(')'))
                        .unwrap_or(var.name)
                        .to_ascii_lowercase();
                    data.insert(
                        name,
                        ComplexSignal {
                            real: v.real.clone(),
                            imag: v.imag.clone(),
                        },
                    );
                }
            }
            results.push(AcData {
                freq: freq.ok_or_else(|| anyhow::anyhow!("missing frequency axis"))?,
                data,
            });
        }
        Ok(results)
    }
}
