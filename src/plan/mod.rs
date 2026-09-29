use crate::blocks::sram::{Sram, SramConfig, SramParams};
use crate::cli::progress::StepContext;
use crate::paths::{out_gds, out_spice, out_verilog};
use crate::verilog::save_1rw_verilog;
use crate::{try_setup_ctx, Result};
use anyhow::{bail, Context};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

static TIMING_DATA: &[(usize, usize, usize, &[u8])] = &[
    (
        64,
        4,
        8,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/timingdata/64m4w8.json"
        )),
    ),
    (
        128,
        4,
        8,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/timingdata/128m4w8.json"
        )),
    ),
    (
        128,
        8,
        8,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/timingdata/128m8w8.json"
        )),
    ),
    (
        256,
        4,
        8,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/timingdata/256m4w8.json"
        )),
    ),
    (
        256,
        8,
        8,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/timingdata/256m8w8.json"
        )),
    ),
    (
        512,
        4,
        8,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/timingdata/512m4w8.json"
        )),
    ),
    (
        512,
        8,
        8,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/timingdata/512m8w8.json"
        )),
    ),
    (
        1024,
        4,
        8,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/timingdata/1024m4w8.json"
        )),
    ),
    (
        1024,
        8,
        8,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/timingdata/1024m8w8.json"
        )),
    ),
    (
        2048,
        4,
        8,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/timingdata/2048m4w8.json"
        )),
    ),
    (
        2048,
        8,
        8,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/timingdata/2048m8w8.json"
        )),
    ),
];

/// A concrete plan for an SRAM.
///
/// Has a 1-1 mapping with a schematic.
#[derive(Clone)]
pub struct SramPlan {
    pub sram_params: SramParams,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum TaskKey {
    GeneratePlan,
    GenerateNetlist,
    GenerateLayout,
    GenerateVerilog,
    GenerateLef,
    #[cfg(feature = "commercial")]
    RunDrc,
    #[cfg(feature = "commercial")]
    RunLvs,
    #[cfg(feature = "commercial")]
    RunPex,
    GenerateLib,
    #[cfg(feature = "commercial")]
    All,
}

pub struct ExecutePlanParams<'a> {
    pub work_dir: &'a Path,
    pub plan: &'a SramPlan,
    pub tasks: Arc<HashSet<TaskKey>>,
    pub ctx: Option<&'a mut StepContext>,
    #[cfg(feature = "commercial")]
    pub pex_level: Option<calibre::pex::PexLevel>,
    #[cfg(feature = "commercial")]
    pub use_liberate: bool,
}

pub fn generate_plan(config: &SramConfig) -> Result<SramPlan> {
    let &SramConfig {
        num_words,
        data_width,
        mux_ratio,
        write_size,
        ..
    } = config;

    if write_size == 0 || data_width == 0 || num_words == 0 {
        bail!("Word count, data width, and write size must be positive");
    }
    if !num_words.is_power_of_two() {
        bail!("Number of words must be a power of two");
    }
    if data_width % write_size != 0 {
        bail!("Data width must be a multiple of write size");
    }

    let params = SramParams::new(write_size, mux_ratio, num_words, data_width);

    if params.rows() < 16 {
        bail!("The number of rows (num words / mux ratio) must be at least 16");
    }

    // Below 16 columns the row decoder's m1 outputs no longer fit between its power
    // straps, among other layout limits.
    if params.cols() < 16 {
        bail!("The number of columns (data width * mux ratio) must be at least 16");
    }

    Ok(SramPlan {
        sram_params: params,
    })
}

/// Every stage that failed, in execution order. The remaining stages still
/// ran, so the other views were written wherever possible.
#[derive(Debug)]
pub struct StageFailures(pub Vec<String>);

impl std::fmt::Display for StageFailures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.join("\n"))
    }
}

impl std::error::Error for StageFailures {}

/// Run one stage. A failure, including a panic, is recorded and later stages
/// still run: a broken view never prevents the others from being written.
fn run_stage(
    ctx: &mut Option<&mut StepContext>,
    key: TaskKey,
    failures: &mut Vec<String>,
    body: impl FnOnce(Option<&StepContext>) -> Result<()>,
) {
    if let Some(ctx) = ctx.as_mut() {
        ctx.start(key);
    }
    let result = crate::cli::catch_generation(|| body(ctx.as_deref()));
    match result {
        Ok(()) => {
            if let Some(ctx) = ctx.as_mut() {
                ctx.finish(key);
            }
        }
        Err(error) => {
            if let Some(ctx) = ctx.as_mut() {
                ctx.fail(key);
            }
            failures.push(format!(
                "{}: {error:#}",
                crate::cli::progress::stage_name(key)
            ));
        }
    }
}

pub fn execute_plan(params: ExecutePlanParams) -> Result<()> {
    let ExecutePlanParams {
        work_dir,
        plan,
        mut ctx,
        ..
    } = params;

    // Everything before the first stage is setup: if it fails, nothing runs.
    std::fs::create_dir_all(work_dir)?;

    let name = &plan.sram_params.name();
    let sctx = try_setup_ctx()?;
    let mut failures = Vec::new();

    let spice_path = out_spice(work_dir, name);
    // Circuit exports use open device names; licensed signoff and timing tasks
    // below continue to generate their own netlists using the commercial PDK.
    #[cfg(feature = "commercial")]
    let export_ctx = crate::try_setup_open_ctx()?;
    #[cfg(feature = "commercial")]
    let netlist_ctx = &export_ctx;
    #[cfg(not(feature = "commercial"))]
    let netlist_ctx = &sctx;
    run_stage(&mut ctx, TaskKey::GenerateNetlist, &mut failures, |_| {
        netlist_ctx
            .write_schematic_to_file::<Sram>(&plan.sram_params, &spice_path)
            .context("failed to write schematic")?;
        crate::spice::make_portable(&spice_path)?;
        Ok(())
    });

    let gds_path = out_gds(work_dir, name);
    run_stage(&mut ctx, TaskKey::GenerateLayout, &mut failures, |_| {
        sctx.write_layout::<Sram>(&plan.sram_params, &gds_path)
            .context("failed to write layout")?;
        Ok(())
    });

    let verilog_path = out_verilog(work_dir, name);
    run_stage(&mut ctx, TaskKey::GenerateVerilog, &mut failures, |_| {
        save_1rw_verilog(&verilog_path, &plan.sram_params)
            .context("failed to write behavioral model")?;
        Ok(())
    });

    // The abstract regenerates the layout in memory, so it is attempted even
    // when writing the GDS failed.
    run_stage(&mut ctx, TaskKey::GenerateLef, &mut failures, |_| {
        crate::abs::write_abstract(
            &sctx,
            &plan.sram_params,
            crate::paths::out_lef(work_dir, name),
        )
        .context("failed to write abstract")?;
        Ok(())
    });

    #[cfg(feature = "commercial")]
    {
        use std::collections::HashMap;

        use rust_decimal::Decimal;
        use rust_decimal_macros::dec;
        use subgeom::bbox::BoundBox;
        use substrate::schematic::netlist::NetlistPurpose;
        use substrate::verification::pex::PexInput;

        let requested = |key| params.tasks.contains(&key) || params.tasks.contains(&TaskKey::All);
        if requested(TaskKey::RunDrc) {
            run_stage(&mut ctx, TaskKey::RunDrc, &mut failures, |_| {
                let drc_work_dir = work_dir.join("drc");
                let output = sctx
                    .write_drc::<Sram>(&plan.sram_params, drc_work_dir)
                    .context("failed to run DRC")?;
                anyhow::ensure!(
                    matches!(
                        output.summary,
                        substrate::verification::drc::DrcSummary::Pass
                    ),
                    "DRC failed"
                );
                Ok(())
            });
        }
        if requested(TaskKey::RunLvs) {
            run_stage(&mut ctx, TaskKey::RunLvs, &mut failures, |_| {
                let lvs_work_dir = work_dir.join("lvs");
                let output = sctx
                    .write_lvs::<Sram>(&plan.sram_params, lvs_work_dir)
                    .context("failed to run LVS")?;
                anyhow::ensure!(
                    matches!(
                        output.summary,
                        substrate::verification::lvs::LvsSummary::Pass
                    ),
                    "LVS failed"
                );
                Ok(())
            });
        }

        let pex_dir = work_dir.join("pex");
        let pex_source_path = out_spice(&pex_dir, "schematic");
        let pex_out_path = out_spice(&pex_dir, "schematic.pex");

        if params.pex_level.is_some() {
            run_stage(&mut ctx, TaskKey::RunPex, &mut failures, |_| {
                sctx.write_schematic_to_file_for_purpose::<Sram>(
                    &plan.sram_params,
                    &pex_source_path,
                    NetlistPurpose::Pex,
                )?;
                let mut opts = HashMap::with_capacity(1);
                opts.insert("level".into(), params.pex_level.unwrap().as_str().into());
                sctx.run_pex(PexInput {
                    work_dir: pex_dir.clone(),
                    layout_path: gds_path.clone(),
                    layout_cell_name: name.clone(),
                    layout_format: substrate::layout::LayoutFormat::Gds,
                    source_paths: vec![pex_source_path.clone()],
                    source_cell_name: name.clone(),
                    pex_netlist_path: pex_out_path.clone(),
                    opts,
                    ground_net: "vss".to_string(),
                })?;
                if !pex_out_path.exists() {
                    bail!(
                        "PEX failed: no output netlist produced at {:?}",
                        pex_out_path
                    );
                }
                Ok(())
            });
        }

        if params.tasks.contains(&TaskKey::GenerateLib) {
            run_stage(&mut ctx, TaskKey::GenerateLib, &mut failures, |progress| {
                if params.use_liberate {
                    let sram_params = plan.sram_params.clone();
                    let source_path = if params.pex_level.is_some() {
                        pex_out_path.clone()
                    } else {
                        let timing_spice_path = out_spice(work_dir, "timing_schematic");
                        sctx.write_schematic_to_file_for_purpose::<Sram>(
                            &sram_params,
                            &timing_spice_path,
                            NetlistPurpose::Timing,
                        )
                        .context("failed to write timing schematic")?;
                        timing_spice_path
                    };

                    let sram = sctx
                        .instantiate_layout::<Sram>(&sram_params)
                        .context("failed to generate layout")?;
                    let brect = sram.brect();
                    let width = Decimal::new(brect.width(), 3);
                    let height = Decimal::new(brect.height(), 3);

                    let mut handles = Vec::new();
                    for (corner, temp, vdd) in [
                        ("tt", 25, dec!(1.8)),
                        ("ss", 100, dec!(1.6)),
                        ("ff", -40, dec!(1.95)),
                    ] {
                        let verilog_path = verilog_path.clone();
                        let work_dir = std::path::PathBuf::from(work_dir);
                        let source_path = source_path.clone();
                        let sram_params = sram_params.clone();
                        let progress = progress.cloned();
                        handles.push(std::thread::spawn(move || {
                            crate::cli::catch_generation(|| -> Result<()> {
                                let suffix = match corner {
                                    "tt" => "tt_025C_1v80",
                                    "ss" => "ss_100C_1v60",
                                    "ff" => "ff_n40C_1v95",
                                    _ => unreachable!(),
                                };
                                let name = format!("{}_{}", sram_params.name(), suffix);
                                let lib_params = liberate_mx::LibParams::builder()
                                    .work_dir(work_dir.join(format!("lib/{suffix}")))
                                    .output_file(crate::paths::out_lib(&work_dir, &name))
                                    .corner(corner)
                                    .width(width)
                                    .height(height)
                                    .user_verilog(verilog_path)
                                    .cell_name(&*sram_params.name())
                                    .num_words(sram_params.num_words())
                                    .data_width(sram_params.data_width())
                                    .addr_width(sram_params.addr_width())
                                    .wmask_width(sram_params.wmask_width())
                                    .mux_ratio(sram_params.mux_ratio())
                                    .has_wmask(true)
                                    .source_paths(vec![source_path])
                                    .vdd(vdd)
                                    .temp(temp)
                                    .build()?;
                                crate::liberate::generate_sram_lib(&lib_params).with_context(
                                    || {
                                        format!(
                                            "failed to write LIB for {suffix}; logs: {}",
                                            lib_params.work_dir.display()
                                        )
                                    },
                                )?;
                                if let Some(ctx) = progress {
                                    ctx.corner_finished(suffix);
                                }
                                Ok(())
                            })
                        }));
                    }
                    let handles: Vec<_> = handles.into_iter().map(|handle| handle.join()).collect();
                    for result in handles {
                        result.map_err(|panic| {
                            anyhow::anyhow!(
                                "LIB worker panicked: {}",
                                crate::cli::panic_message(&*panic)
                            )
                        })??;
                    }
                } else {
                    generate_interpolated_lib(work_dir, &plan.sram_params, progress)?;
                }
                Ok(())
            });
        }
    }

    #[cfg(not(feature = "commercial"))]
    if params.tasks.contains(&TaskKey::GenerateLib) {
        run_stage(&mut ctx, TaskKey::GenerateLib, &mut failures, |progress| {
            generate_interpolated_lib(work_dir, &plan.sram_params, progress)
        });
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(StageFailures(failures).into())
    }
}

pub(crate) fn interpolated_timing_data(sram_params: &SramParams) -> Result<&'static [u8]> {
    if sram_params.data_width() > 128 {
        anyhow::bail!(
            "open-source lib generation requires data_width ≤ 128 (got {})",
            sram_params.data_width()
        );
    }
    let nw = sram_params.num_words();
    let mx = sram_params.mux_ratio();
    let ws = sram_params.wmask_granularity();
    let json_bytes: &[u8] = TIMING_DATA
        .iter()
        .find(|(n, m, w, _)| *n == nw && *m == mx && *w == ws)
        .map(|(_, _, _, b)| *b)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no timing data for {}m{}w{} — add timingdata/{}m{}w{}.json to the repo",
                nw,
                mx,
                ws,
                nw,
                mx,
                ws
            )
        })?;
    Ok(json_bytes)
}

fn generate_interpolated_lib(
    work_dir: &Path,
    sram_params: &SramParams,
    ctx: Option<&StepContext>,
) -> Result<()> {
    use crate::liberty::{LibGenParams, LookupModel, PvtCorner};
    let json_bytes = interpolated_timing_data(sram_params)?;
    let name = sram_params.name();
    for pvt in [PvtCorner::tt(), PvtCorner::ss(), PvtCorner::ff()] {
        let model = LookupModel::from_json(json_bytes, &pvt.name)?;
        let suffix = pvt.file_suffix();
        let lib_name = format!("{}_{}", name, suffix);
        let lib_path = crate::paths::out_lib(work_dir, &lib_name);
        crate::liberty::generate_sram_lib(&LibGenParams {
            sram: sram_params,
            pvt,
            model: &model,
            output: lib_path,
        })?;
        if let Some(ctx) = ctx {
            ctx.corner_finished(suffix);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::progress::Event;

    #[test]
    fn a_failed_stage_does_not_stop_later_stages() {
        let (events, receiver) = std::sync::mpsc::channel();
        let mut step = StepContext::new(7, events);
        let mut ctx = Some(&mut step);
        let mut failures = Vec::new();
        run_stage(
            &mut ctx,
            TaskKey::GenerateNetlist,
            &mut failures,
            |_| Ok(()),
        );
        run_stage(&mut ctx, TaskKey::GenerateLayout, &mut failures, |_| {
            bail!("layout failed")
        });
        run_stage(&mut ctx, TaskKey::GenerateVerilog, &mut failures, |_| {
            panic!("verilog panic")
        });
        run_stage(&mut ctx, TaskKey::GenerateLef, &mut failures, |progress| {
            assert!(progress.is_some());
            Ok(())
        });
        assert_eq!(
            failures,
            [
                "GDS: layout failed",
                "Verilog: generation panicked: verilog panic"
            ]
        );
        assert_eq!(
            StageFailures(failures).to_string(),
            "GDS: layout failed\nVerilog: generation panicked: verilog panic"
        );
        drop(step);
        let outcomes: Vec<_> = receiver
            .iter()
            .filter_map(|event| match event {
                Event::StageFinished { id: 7, key } => Some((key, true)),
                Event::StageFailed { id: 7, key } => Some((key, false)),
                _ => None,
            })
            .collect();
        assert_eq!(
            outcomes,
            [
                (TaskKey::GenerateNetlist, true),
                (TaskKey::GenerateLayout, false),
                (TaskKey::GenerateVerilog, false),
                (TaskKey::GenerateLef, true),
            ]
        );
    }

    #[test]
    fn invalid_dimensions_return_errors_without_panicking_or_truncating() {
        for (words, width, write_size) in [
            (0, 8, 8),
            (1, 8, 8),
            (65, 8, 8),
            (64, 0, 8),
            (64, 8, 0),
            (64, 8, 3),
            (64, 2, 2),
        ] {
            let config: SramConfig = toml::from_str(&format!(
                "num_words={words}\ndata_width={width}\nwrite_size={write_size}\nmux_ratio=4\n"
            ))
            .unwrap();
            assert!(generate_plan(&config).is_err());
        }
        for (words, width, write_size, mux) in [(64, 8, 8, 4), (64, 4, 4, 4), (128, 2, 1, 8)] {
            let config: SramConfig = toml::from_str(&format!(
                "num_words={words}\ndata_width={width}\nwrite_size={write_size}\nmux_ratio={mux}\n"
            ))
            .unwrap();
            assert!(generate_plan(&config).is_ok());
        }
    }
}
