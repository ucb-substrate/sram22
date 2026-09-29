use std::any::Any;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::fs::canonicalize;
use std::io::{BufRead, BufReader};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use clap::Parser;

use crate::blocks::sram::{parse_sram_batch_config, SramConfig};
use crate::cli::args::Args;
use crate::cli::progress::{Event, Job, Level, Options, Reporter, StepContext};
use crate::paths::{out_gds, out_lef, out_lib, out_spice, out_verilog};
use crate::plan::{
    execute_plan, generate_plan, interpolated_timing_data, ExecutePlanParams, SramPlan,
    StageFailures, TaskKey,
};
use crate::Result;

pub mod args;
pub mod progress;

/// The CLI has already printed these diagnostics; the binary only needs to set
/// its exit status. Other errors are printed by main with their context chain.
#[doc(hidden)]
#[derive(Debug, thiserror::Error)]
#[error("SRAM generation failed; see diagnostics above")]
pub struct ReportedError;

fn is_already_built(work_dir: &std::path::Path, name: &str) -> bool {
    out_spice(work_dir, name).exists()
        && out_gds(work_dir, name).exists()
        && out_verilog(work_dir, name).exists()
        && out_lef(work_dir, name).exists()
        && ["tt_025C_1v80", "ss_100C_1v60", "ff_n40C_1v95"]
            .iter()
            .all(|suffix| out_lib(work_dir, &format!("{name}_{suffix}")).exists())
        && std::fs::File::open(out_spice(work_dir, name))
            .map(|file| {
                BufReader::new(file).lines().all(|line| {
                    line.is_ok_and(|line| !crate::spice::is_external_or_model_directive(&line))
                })
            })
            .unwrap_or(false)
}

#[cfg(feature = "commercial")]
fn config_tasks(base: &HashSet<TaskKey>, config: &SramConfig) -> Arc<HashSet<TaskKey>> {
    let mut tasks = base.clone();
    if config.pex_level.is_some() {
        tasks.insert(TaskKey::RunPex);
    }
    Arc::new(tasks)
}

#[cfg(not(feature = "commercial"))]
fn config_tasks(base: &HashSet<TaskKey>, _config: &SramConfig) -> Arc<HashSet<TaskKey>> {
    Arc::new(base.clone())
}

fn validated_plans(configs: &[SramConfig]) -> Result<Vec<SramPlan>> {
    let mut plans = Vec::new();
    let mut names = HashMap::new();
    let mut errors = Vec::new();
    for (index, config) in configs.iter().enumerate() {
        match generate_plan(config) {
            Ok(plan) => {
                let name = plan.sram_params.name();
                if let Some(previous) = names.insert(name.clone(), index + 1) {
                    errors.push(format!("entry {} ({name}): duplicates entry {previous}; both would write to the same output directory", index + 1));
                }
                plans.push(plan);
            }
            Err(error) => errors.push(format!("entry {}: {error:#}", index + 1)),
        }
    }
    anyhow::ensure!(
        errors.is_empty(),
        "Invalid SRAM configuration:\n  {}",
        errors.join("\n  ")
    );
    Ok(plans)
}

pub fn run() -> Result<()> {
    let args = Args::parse();
    anyhow::ensure!(
        args.parallel != Some(0),
        "--parallel must be greater than zero"
    );
    let config_path = canonicalize(&args.config)
        .with_context(|| format!("cannot open configuration {}", args.config.display()))?;
    let configs = parse_sram_batch_config(&config_path)
        .with_context(|| format!("invalid configuration {}", config_path.display()))?;
    let plans = validated_plans(&configs)?;
    let build_dir = args
        .output_dir
        .clone()
        .unwrap_or_else(|| config_path.parent().unwrap().join("build"));

    let base_tasks: HashSet<TaskKey> = [
        (true, TaskKey::GenerateLib),
        #[cfg(feature = "commercial")]
        (args.drc, TaskKey::RunDrc),
        #[cfg(feature = "commercial")]
        (args.lvs, TaskKey::RunLvs),
        #[cfg(feature = "commercial")]
        (args.all, TaskKey::All),
    ]
    .into_iter()
    .filter_map(|(enabled, task)| enabled.then_some(task))
    .collect();

    let mut reused = Vec::new();
    let mut warnings = Vec::new();
    for (plan, _config) in plans.iter().zip(&configs) {
        let name = plan.sram_params.name();
        #[cfg(not(feature = "commercial"))]
        let must_run = args.force;
        #[cfg(feature = "commercial")]
        let must_run = args.force
            || args.liberate
            || args.drc
            || args.lvs
            || args.all
            || _config.pex_level.is_some();
        let reuse = !must_run && is_already_built(&build_dir.join(name.as_str()), &name);
        reused.push(reuse);
        #[cfg(not(feature = "commercial"))]
        let interpolate = true;
        #[cfg(feature = "commercial")]
        let interpolate = !args.liberate;
        // Only LIB depends on timing data; the other views are still generated.
        if !reuse && interpolate {
            if let Err(error) = interpolated_timing_data(&plan.sram_params) {
                warnings.push(format!("{name}: LIB cannot be generated: {error:#}"));
            }
        }
    }

    // Preflight runs before creating output directories or starting any work.
    std::fs::create_dir_all(&build_dir)
        .with_context(|| format!("cannot create output directory {}", build_dir.display()))?;
    let build_dir = canonicalize(build_dir)?;
    let jobs = plans
        .iter()
        .zip(&configs)
        .enumerate()
        .map(|(id, (plan, config))| {
            let name = plan.sram_params.name().to_string();
            Job::new(
                name.clone(),
                build_dir.join(&name),
                &config_tasks(&base_tasks, config),
                reused[id],
            )
        })
        .collect();
    let work_items: Vec<_> = plans
        .into_iter()
        .zip(configs)
        .enumerate()
        .filter(|(id, _)| !reused[*id])
        .collect();
    let num_workers = args
        .parallel
        .unwrap_or(work_items.len())
        .min(work_items.len());
    let mut reporter = Reporter::new(jobs, Options::from_args(&args));
    reporter.announce(&config_path, &build_dir, num_workers);
    for warning in &warnings {
        reporter.message("Warning", warning, Level::Warning);
    }
    reporter.redraw();

    let _panic_hook = WorkerPanicHook::install();
    let queue = Arc::new(Mutex::new(work_items.into_iter()));
    let (events, receiver) = mpsc::channel();
    let mut workers = Vec::new();
    for worker_id in 0..num_workers {
        let queue = Arc::clone(&queue);
        let events = events.clone();
        let base_tasks = base_tasks.clone();
        let build_dir = build_dir.clone();
        #[cfg(feature = "commercial")]
        let use_liberate = args.liberate;
        match std::thread::Builder::new()
            .name(format!("sram22-{worker_id}"))
            .spawn(move || loop {
                let item = queue.lock().unwrap().next();
                let Some((id, (plan, config))) = item else {
                    break;
                };
                run_job(id, events.clone(), |ctx| {
                    let tasks = config_tasks(&base_tasks, &config);
                    let work_dir = build_dir.join(plan.sram_params.name().as_str());
                    std::fs::create_dir_all(&work_dir)
                        .with_context(|| format!("cannot create {}", work_dir.display()))?;
                    execute_plan(ExecutePlanParams {
                        work_dir: &work_dir,
                        plan: &plan,
                        tasks,
                        ctx: Some(ctx),
                        #[cfg(feature = "commercial")]
                        pex_level: config.pex_level,
                        #[cfg(feature = "commercial")]
                        use_liberate,
                    })
                });
            }) {
            Ok(worker) => workers.push(worker),
            Err(error) => {
                reporter.message(
                    "Warning",
                    &format!("could only start {} workers: {error}", workers.len()),
                    Level::Warning,
                );
                break;
            }
        }
    }
    drop(events);
    loop {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                reporter.handle(event);
                // Coalesce bursts from fast stages without starving redraws.
                for event in receiver.try_iter().take(128) {
                    reporter.handle(event);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        reporter.redraw();
    }
    let mut worker_failed = false;
    for worker in workers {
        if let Err(panic) = worker.join() {
            worker_failed = true;
            reporter.message(
                "Failed",
                &format!("worker panicked: {}", panic_message(&*panic)),
                Level::Error,
            );
        }
    }
    let failures = reporter.finish(&build_dir);
    if failures > 0 || worker_failed {
        return Err(ReportedError.into());
    }
    Ok(())
}

fn run_job(
    id: usize,
    events: mpsc::Sender<Event>,
    run: impl FnOnce(&mut StepContext) -> Result<()>,
) {
    let started = Instant::now();
    let _ = events.send(Event::Started { id, at: started });
    let mut ctx = StepContext::new(id, events.clone());
    let result = catch_generation(|| run(&mut ctx)).map_err(|error| {
        if error.is::<StageFailures>() {
            // Each failure already names its stage; the other views were written.
            error.to_string()
        } else {
            format!("{}: {error:#}", ctx.current_stage())
        }
    });
    let _ = events.send(Event::Finished {
        id,
        elapsed: started.elapsed(),
        result,
    });
}

pub(crate) fn panic_message(panic: &(dyn Any + Send)) -> &str {
    panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&'static str>().copied())
        .unwrap_or("no panic message")
}

thread_local! {
    static IN_GENERATION: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn catch_generation<T>(run: impl FnOnce() -> Result<T>) -> Result<T> {
    let previous = IN_GENERATION.replace(true);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run));
    IN_GENERATION.set(previous);
    result.unwrap_or_else(|panic| {
        Err(anyhow::anyhow!(
            "generation panicked: {}",
            panic_message(&*panic)
        ))
    })
}

type PanicHook = dyn Fn(&std::panic::PanicHookInfo<'_>) + Send + Sync + 'static;

// Caught generator panics are reported through the same event stream as errors.
// Preserve the existing hook for unrelated threads and restore it after the run.
struct WorkerPanicHook(Arc<PanicHook>);

impl WorkerPanicHook {
    fn install() -> Self {
        let previous: Arc<PanicHook> = Arc::from(std::panic::take_hook());
        let hook = previous.clone();
        std::panic::set_hook(Box::new(move |info| {
            if !IN_GENERATION.get() {
                hook(info);
            }
        }));
        Self(previous)
    }
}

impl Drop for WorkerPanicHook {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let previous = self.0.clone();
            std::panic::set_hook(Box::new(move |info| previous(info)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_validation_reports_entry_numbers_and_output_collisions() {
        let config: SramConfig =
            toml::from_str("num_words=64\ndata_width=8\nwrite_size=8\nmux_ratio=4\n").unwrap();
        let error = validated_plans(&[config.clone(), config.clone()])
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("entry 2") && error.contains("duplicates entry 1"));
        let mut invalid = config.clone();
        invalid.num_words = 65;
        let error = validated_plans(&[config, invalid])
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("entry 2") && error.contains("power of two"));
    }

    #[test]
    fn every_concurrent_job_reports_one_result_including_errors_and_panics() {
        let (events, receiver) = mpsc::channel();
        let workers: Vec<_> = (0..4)
            .map(|id| {
                let events = events.clone();
                std::thread::spawn(move || {
                    run_job(id, events, |ctx| {
                        if id == 0 {
                            anyhow::bail!("cannot create work directory");
                        }
                        ctx.start(TaskKey::GenerateLayout);
                        if id == 1 {
                            panic!("layout panic");
                        }
                        if id == 2 {
                            anyhow::bail!("layout error");
                        }
                        ctx.finish(TaskKey::GenerateLayout);
                        Ok(())
                    })
                })
            })
            .collect();
        drop(events);
        for worker in workers {
            worker.join().unwrap();
        }
        let outcomes: Vec<_> = receiver
            .into_iter()
            .filter_map(|event| {
                if let Event::Finished { id, result, .. } = event {
                    Some((id, result))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(outcomes.len(), 4);
        let outcomes: HashMap<_, _> = outcomes.into_iter().collect();
        assert_eq!(outcomes.len(), 4);
        assert_eq!(
            outcomes[&0].as_ref().unwrap_err(),
            "Setup: cannot create work directory"
        );
        assert_eq!(
            outcomes[&1].as_ref().unwrap_err(),
            "GDS: generation panicked: layout panic"
        );
        assert_eq!(outcomes[&2].as_ref().unwrap_err(), "GDS: layout error");
        assert!(outcomes[&3].is_ok());
    }

    #[test]
    fn completed_generation_requires_all_views_and_a_portable_circuit() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let name = "sram22_64x8m4w8";
        for extension in ["gds", "lef", "v"] {
            std::fs::write(root.join(format!("{name}.{extension}")), "generated").unwrap();
        }
        std::fs::write(
            out_spice(root, name),
            ".subckt sram22_64x8m4w8 clk vdd vss\n.ends\n",
        )
        .unwrap();
        assert!(!is_already_built(root, name));
        for suffix in ["tt_025C_1v80", "ss_100C_1v60", "ff_n40C_1v95"] {
            std::fs::write(out_lib(root, &format!("{name}_{suffix}")), "generated").unwrap();
        }
        assert!(is_already_built(root, name));
        std::fs::remove_file(out_lib(root, &format!("{name}_ss_100C_1v60"))).unwrap();
        assert!(!is_already_built(root, name));
        std::fs::write(out_lib(root, &format!("{name}_ss_100C_1v60")), "generated").unwrap();
        for directive in [
            ".model nfet nmos level=1",
            ".include /tmp/cells.spice",
            ".lib /tmp/models.spice tt",
        ] {
            std::fs::write(out_spice(root, name), directive).unwrap();
            assert!(!is_already_built(root, name));
        }
    }
}
