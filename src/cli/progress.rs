//! Workers send events; only the reporter owns terminal state.
use std::collections::HashSet;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use console::{measure_text_width, truncate_str, Term};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::args::{Args, ColorMode, ProgressMode};
use crate::plan::TaskKey;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Width of the right-aligned action label on permanent messages, cargo-style.
const ACTION_WIDTH: usize = 11;

pub(crate) fn stage_name(key: TaskKey) -> &'static str {
    match key {
        TaskKey::GeneratePlan => "Plan",
        TaskKey::GenerateNetlist => "SPICE",
        TaskKey::GenerateLayout => "GDS",
        TaskKey::GenerateVerilog => "Verilog",
        TaskKey::GenerateLef => "LEF",
        TaskKey::GenerateLib => "LIB",
        #[cfg(feature = "commercial")]
        TaskKey::RunDrc => "DRC",
        #[cfg(feature = "commercial")]
        TaskKey::RunLvs => "LVS",
        #[cfg(feature = "commercial")]
        TaskKey::RunPex => "PEX",
        #[cfg(feature = "commercial")]
        TaskKey::All => "All",
    }
}

pub(crate) fn stages(tasks: &HashSet<TaskKey>) -> Vec<TaskKey> {
    let mut stages = vec![
        TaskKey::GenerateNetlist,
        TaskKey::GenerateLayout,
        TaskKey::GenerateVerilog,
        TaskKey::GenerateLef,
    ];
    #[cfg(feature = "commercial")]
    for key in [TaskKey::RunDrc, TaskKey::RunLvs, TaskKey::RunPex] {
        if tasks.contains(&key) || (key != TaskKey::RunPex && tasks.contains(&TaskKey::All)) {
            stages.push(key);
        }
    }
    if tasks.contains(&TaskKey::GenerateLib) {
        stages.push(TaskKey::GenerateLib);
    }
    stages
}

#[derive(Debug)]
pub(crate) enum Event {
    Started {
        id: usize,
        at: Instant,
    },
    StageStarted {
        id: usize,
        key: TaskKey,
    },
    StageFinished {
        id: usize,
        key: TaskKey,
    },
    CornerFinished {
        id: usize,
        corner: String,
    },
    Finished {
        id: usize,
        elapsed: Duration,
        result: Result<(), String>,
    },
}

/// A job-local event sender. Stage transitions follow execution, rather than
/// advancing through a second, implicitly ordered copy of the plan.
#[derive(Clone)]
pub struct StepContext {
    id: usize,
    events: Sender<Event>,
    current: Option<TaskKey>,
}

impl StepContext {
    pub(crate) fn new(id: usize, events: Sender<Event>) -> Self {
        Self {
            id,
            events,
            current: None,
        }
    }

    pub fn start(&mut self, key: TaskKey) {
        self.current = Some(key);
        let _ = self.events.send(Event::StageStarted { id: self.id, key });
    }

    pub fn finish(&mut self, key: TaskKey) {
        let _ = self.events.send(Event::StageFinished { id: self.id, key });
        if self.current == Some(key) {
            self.current = None;
        }
    }

    pub fn corner_finished(&self, corner: impl Into<String>) {
        let _ = self.events.send(Event::CornerFinished {
            id: self.id,
            corner: corner.into(),
        });
    }

    pub(crate) fn current_stage(&self) -> &'static str {
        self.current.map(stage_name).unwrap_or("Setup")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StageState {
    Pending,
    Running,
    Done,
    Failed,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JobStatus {
    Queued,
    Running,
    Done,
    Reused,
    Failed,
}

pub(crate) struct Job {
    pub name: String,
    pub work_dir: PathBuf,
    stages: Vec<(TaskKey, StageState)>,
    status: JobStatus,
    started: Option<Instant>,
    elapsed: Duration,
    corners: HashSet<String>,
}

impl Job {
    pub(crate) fn new(
        name: String,
        work_dir: PathBuf,
        tasks: &HashSet<TaskKey>,
        reused: bool,
    ) -> Self {
        Self {
            name,
            work_dir,
            stages: stages(tasks)
                .into_iter()
                .map(|key| {
                    (
                        key,
                        if reused {
                            StageState::Done
                        } else {
                            StageState::Pending
                        },
                    )
                })
                .collect(),
            status: if reused {
                JobStatus::Reused
            } else {
                JobStatus::Queued
            },
            started: None,
            elapsed: Duration::ZERO,
            corners: HashSet::new(),
        }
    }

    fn stage(&self, key: TaskKey) -> Option<StageState> {
        self.stages
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, state)| *state)
    }

    fn set_stage(&mut self, key: TaskKey, state: StageState) {
        if let Some((_, current)) = self.stages.iter_mut().find(|(k, _)| *k == key) {
            *current = state;
        }
    }

    fn elapsed(&self) -> Duration {
        if self.status == JobStatus::Running {
            self.started.map(|at| at.elapsed()).unwrap_or_default()
        } else {
            self.elapsed
        }
    }

    /// Text for the status column and the color code it is painted with.
    fn status(&self) -> (String, &'static str) {
        match self.status {
            JobStatus::Queued => ("queued".into(), "2"),
            JobStatus::Reused => ("reused".into(), ""),
            JobStatus::Failed => ("failed".into(), "1;31"),
            JobStatus::Running if self.stages.iter().all(|(_, s)| *s == StageState::Pending) => {
                ("setup".into(), "")
            }
            _ => (duration(self.elapsed()), ""),
        }
    }
}

/// Severity of a permanent message. Only errors bypass `--quiet`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    Info,
    Success,
    Warning,
    Error,
}

impl Level {
    fn code(self) -> &'static str {
        match self {
            Level::Info => "1;36",
            Level::Success => "1;32",
            Level::Warning => "1;33",
            Level::Error => "1;31",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Options {
    pub progress: ProgressMode,
    pub color: bool,
    pub verbose: bool,
    pub quiet: bool,
    pub live: bool,
}

impl Options {
    pub(crate) fn from_args(args: &Args) -> Self {
        let terminal = io::stderr().is_terminal()
            && std::env::var_os("TERM").is_none_or(|term| term != "dumb");
        Self {
            progress: args.progress,
            color: match args.color {
                ColorMode::Always => true,
                ColorMode::Never => false,
                ColorMode::Auto => terminal && std::env::var_os("NO_COLOR").is_none(),
            },
            verbose: args.verbose,
            quiet: args.quiet,
            live: terminal && args.progress == ProgressMode::Auto && !args.quiet,
        }
    }

    /// Wrap `text` in an SGR sequence; an empty code leaves it unpainted.
    fn paint(&self, text: &str, code: &str) -> String {
        if self.color && !code.is_empty() {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.into()
        }
    }

    fn updates(&self) -> bool {
        !self.quiet && self.progress != ProgressMode::Off
    }
}

struct LiveDisplay {
    mp: MultiProgress,
    table: ProgressBar,
}

impl LiveDisplay {
    fn new() -> Self {
        let mp = MultiProgress::new();
        let table = mp.add(ProgressBar::new_spinner());
        table.set_style(ProgressStyle::with_template("{msg}").expect("static progress template"));
        Self { mp, table }
    }
}

impl Drop for LiveDisplay {
    fn drop(&mut self) {
        self.table.finish_and_clear();
    }
}

#[derive(Default)]
struct Counts {
    done: usize,
    reused: usize,
    failed: usize,
    running: usize,
    queued: usize,
}

/// Owns the entire live area, including permanent messages printed above it.
pub(crate) struct Reporter {
    jobs: Vec<Job>,
    options: Options,
    live: Option<LiveDisplay>,
    started: Instant,
}

impl Reporter {
    pub(crate) fn new(jobs: Vec<Job>, options: Options) -> Self {
        let live = options.live.then(LiveDisplay::new);
        Self {
            jobs,
            options,
            live,
            started: Instant::now(),
        }
    }

    /// Print a permanent line: a right-aligned action label followed by text.
    /// Continuation lines in `text` are indented to align under the first.
    pub(crate) fn message(&self, action: &str, text: &str, level: Level) {
        let label = self
            .options
            .paint(&format!("{action:>ACTION_WIDTH$}"), level.code());
        let text = text.replace('\n', &format!("\n{:ACTION_WIDTH$} ", ""));
        self.write_line(&format!("{label} {text}"), level == Level::Error);
    }

    fn write_line(&self, line: &str, error: bool) {
        if self.options.quiet && !error {
            return;
        }
        if let Some(live) = &self.live {
            // indicatif ignores an empty string; retain intentional spacing.
            let _ = live.mp.println(if line.is_empty() { " " } else { line });
        } else {
            let _ = writeln!(io::stderr().lock(), "{line}");
        }
    }

    pub(crate) fn announce(&self, config: &Path, output: &Path, workers: usize) {
        let plural = |count: usize, noun: &str| {
            format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
        };
        self.message(
            "SRAM22",
            &format!(
                "v{} · {} · {}",
                env!("CARGO_PKG_VERSION"),
                plural(self.jobs.len(), "SRAM"),
                plural(workers, "worker")
            ),
            Level::Success,
        );
        self.message("Config", &config.display().to_string(), Level::Info);
        self.message("Output", &output.display().to_string(), Level::Info);
        self.write_line("", false);
        for (id, job) in self.jobs.iter().enumerate() {
            if self.options.verbose {
                let plan = job
                    .stages
                    .iter()
                    .map(|(k, _)| stage_name(*k))
                    .collect::<Vec<_>>()
                    .join(" → ");
                self.message(
                    "Plan",
                    &format!("[{}] {}: {plan}", id + 1, job.name),
                    Level::Info,
                );
            }
            if job.status == JobStatus::Reused && self.options.updates() {
                self.message("Reused", &job.name, Level::Success);
                if self.options.verbose {
                    self.message(
                        "Artifacts",
                        &job.work_dir.display().to_string(),
                        Level::Info,
                    );
                }
            }
        }
    }

    pub(crate) fn handle(&mut self, event: Event) {
        match event {
            Event::Started { id, at } => {
                let job = &mut self.jobs[id];
                job.status = JobStatus::Running;
                job.started = Some(at);
                if self.options.updates() && (self.live.is_none() || self.options.verbose) {
                    self.message("Started", &self.jobs[id].name, Level::Info);
                }
            }
            Event::StageStarted { id, key } => {
                self.jobs[id].set_stage(key, StageState::Running);
                if self.options.updates() && self.options.verbose {
                    self.message(stage_name(key), &self.jobs[id].name, Level::Info);
                }
            }
            Event::StageFinished { id, key } => self.jobs[id].set_stage(key, StageState::Done),
            Event::CornerFinished { id, corner } => {
                let job = &mut self.jobs[id];
                job.corners.insert(corner.clone());
                if self.options.updates() && self.options.verbose {
                    self.message(
                        "LIB",
                        &format!("{}: {corner} complete", self.jobs[id].name),
                        Level::Info,
                    );
                }
            }
            Event::Finished {
                id,
                elapsed,
                result,
            } => {
                let job = &mut self.jobs[id];
                job.elapsed = elapsed;
                match result {
                    Ok(()) => {
                        job.status = JobStatus::Done;
                        // A successful job completed every stage it started; do
                        // not leave a spinner on a finished row.
                        for (_, state) in &mut job.stages {
                            if *state == StageState::Running {
                                *state = StageState::Done;
                            }
                        }
                        if self.options.updates() {
                            self.message(
                                "Finished",
                                &format!("{} in {}", self.jobs[id].name, duration(elapsed)),
                                Level::Success,
                            );
                            if self.options.verbose {
                                self.message(
                                    "Artifacts",
                                    &self.jobs[id].work_dir.display().to_string(),
                                    Level::Info,
                                );
                            }
                        }
                    }
                    Err(error) => {
                        job.status = JobStatus::Failed;
                        for (_, state) in &mut job.stages {
                            *state = match *state {
                                StageState::Running => StageState::Failed,
                                StageState::Pending => StageState::Blocked,
                                state => state,
                            };
                        }
                        self.message(
                            "Failed",
                            &format!(
                                "{} after {}\n{error}\nWork directory: {}",
                                self.jobs[id].name,
                                duration(elapsed),
                                self.jobs[id].work_dir.display()
                            ),
                            Level::Error,
                        );
                    }
                }
            }
        }
    }

    fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for job in &self.jobs {
            match job.status {
                JobStatus::Queued => counts.queued += 1,
                JobStatus::Running => counts.running += 1,
                JobStatus::Done => counts.done += 1,
                JobStatus::Reused => counts.reused += 1,
                JobStatus::Failed => counts.failed += 1,
            }
        }
        counts
    }

    pub(crate) fn redraw(&self) {
        if let Some(live) = &self.live {
            let (height, width) = Term::stderr().size();
            live.table
                .set_message(self.render(width as usize, height as usize));
        }
    }

    fn cell(&self, job: &Job, key: TaskKey, spinner: &str) -> (String, &'static str) {
        match job.stage(key) {
            Some(StageState::Done) => ("✓".into(), "32"),
            Some(StageState::Running) if key == TaskKey::GenerateLib => {
                (format!("{spinner} {}/3", job.corners.len()), "1;36")
            }
            Some(StageState::Running) => (spinner.into(), "1;36"),
            Some(StageState::Pending) => ("·".into(), "2"),
            Some(StageState::Failed) => ("✗".into(), "1;31"),
            Some(StageState::Blocked) => ("!".into(), "33"),
            None => ("—".into(), "2"),
        }
    }

    /// Pure layout, measured in terminal cells. Reserve a screen line to avoid
    /// autowrap/scrolling; oversized batches prioritize active jobs, then queued.
    fn render(&self, width: usize, height: usize) -> String {
        self.render_at(width, height, self.started.elapsed())
    }

    fn render_at(&self, width: usize, height: usize, elapsed: Duration) -> String {
        // The reporter redraws every 100 ms, even when workers have no new
        // events. Sample one frame so every active cell stays in sync.
        let spinner =
            SPINNER_FRAMES[(elapsed.as_millis() / 100 % SPINNER_FRAMES.len() as u128) as usize];
        let width = width.saturating_sub(1);
        let height = height.saturating_sub(1);
        if width == 0 || height == 0 {
            return String::new();
        }
        let counts = self.counts();
        let complete = counts.done + counts.reused + counts.failed;
        let failures = if counts.failed > 0 {
            format!(" · {} failed", counts.failed)
        } else {
            String::new()
        };
        let summary = format!(
            "Building {complete}/{} complete · {} running · {} queued{failures} · {}",
            self.jobs.len(),
            counts.running,
            counts.queued,
            duration(elapsed)
        );
        let mut lines = if width >= measure_text_width(&summary) {
            vec![summary]
        } else {
            vec![
                format!(
                    "Building {complete}/{} complete · {}",
                    self.jobs.len(),
                    duration(elapsed)
                ),
                format!(
                    "{} running · {} queued{failures}",
                    counts.running, counts.queued
                ),
            ]
        };
        let columns: HashSet<_> = self
            .jobs
            .iter()
            .flat_map(|job| job.stages.iter().map(|(key, _)| *key))
            .collect::<HashSet<_>>();
        // Use execution order, not the iteration order of a HashSet.
        let columns = stages(&columns);
        let name_width = self
            .jobs
            .iter()
            .map(|j| j.name.len())
            .max()
            .unwrap_or(4)
            .max(4);
        let column_width = |key| {
            stage_name(key)
                .len()
                .max(if key == TaskKey::GenerateLib { 5 } else { 1 })
        };
        let table_width =
            name_width + 2 + columns.iter().map(|&k| column_width(k) + 2).sum::<usize>() + 8;
        let wide = table_width <= width;
        if height >= 8 {
            lines.push(String::new());
        }
        let mut header = format!("{:name_width$}  ", "SRAM");
        if wide {
            for &key in &columns {
                header.push_str(&format!("{:^w$}  ", stage_name(key), w = column_width(key)));
            }
            header.push_str(" Elapsed");
            lines.push(self.options.paint(&header, "1"));
            if height >= 8 {
                lines.push(self.options.paint(&"─".repeat(table_width), "2"));
            }
        }
        let rows: Vec<Vec<String>> = self
            .jobs
            .iter()
            .map(|job| {
                let (status, status_color) = job.status();
                if wide {
                    let mut row = format!("{:name_width$}  ", job.name);
                    for &key in &columns {
                        let (cell, color) = self.cell(job, key, spinner);
                        row.push_str(
                            &self
                                .options
                                .paint(&format!("{:^w$}", cell, w = column_width(key)), color),
                        );
                        row.push_str("  ");
                    }
                    row.push_str(&self.options.paint(&format!("{status:>8}"), status_color));
                    vec![row]
                } else {
                    let label =
                        truncate_str(&job.name, width.saturating_sub(status.len() + 2), "…");
                    let mut rows = vec![format!(
                        "{label}  {}",
                        self.options.paint(&status, status_color)
                    )];
                    let mut line = String::from("  ");
                    for &(key, _) in &job.stages {
                        let (cell, color) = self.cell(job, key, spinner);
                        let token = format!("{cell} {}", stage_name(key));
                        if measure_text_width(&line) + measure_text_width(&token) > width
                            && line.len() > 2
                        {
                            rows.push(line.trim_end().into());
                            line = "  ".into();
                        }
                        line.push_str(&self.options.paint(&token, color));
                        line.push_str("  ");
                    }
                    rows.push(line.trim_end().into());
                    rows
                }
            })
            .collect();
        // Spinners are self-explanatory; "—" only appears when jobs differ in
        // which optional stages they requested.
        let unused = self
            .jobs
            .iter()
            .any(|job| columns.iter().any(|&key| job.stage(key).is_none()));
        let legend = if unused {
            "✓ done  · pending  ✗ failed  ! blocked  — not requested"
        } else {
            "✓ done  · pending  ✗ failed  ! blocked"
        };
        // The legend takes two lines: a separating blank line and itself.
        let show_legend = width >= measure_text_width(legend) && height >= lines.len() + 5;
        let available = height.saturating_sub(lines.len() + if show_legend { 2 } else { 0 });
        if rows.iter().map(Vec::len).sum::<usize>() <= available {
            for row in rows {
                lines.extend(row);
            }
        } else {
            let mut remaining = available.saturating_sub(1); // hidden-job counts
            let mut selected = vec![false; self.jobs.len()];
            for status in [JobStatus::Running, JobStatus::Queued] {
                for (id, job) in self.jobs.iter().enumerate() {
                    if job.status == status && rows[id].len() <= remaining {
                        selected[id] = true;
                        remaining -= rows[id].len();
                    }
                }
            }
            let mut hidden_running = 0;
            let mut hidden_queued = 0;
            for (id, job) in self.jobs.iter().enumerate() {
                if selected[id] {
                    lines.extend(rows[id].clone());
                } else if job.status == JobStatus::Running {
                    hidden_running += 1;
                } else if job.status == JobStatus::Queued {
                    hidden_queued += 1;
                }
            }
            if available > 0 {
                lines.push(self.options.paint(
                    &format!("+{hidden_running} running, {hidden_queued} queued not shown"),
                    "2",
                ));
            }
        }
        if show_legend {
            lines.push(String::new());
            lines.push(self.options.paint(legend, "2"));
        }
        lines
            .into_iter()
            .take(height)
            .map(|line| truncate_str(&line, width, "…").into_owned())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Finalize even if a worker exited without sending its terminal event.
    pub(crate) fn finish(&mut self, output: &Path) -> usize {
        for id in 0..self.jobs.len() {
            if matches!(self.jobs[id].status, JobStatus::Queued | JobStatus::Running) {
                self.handle(Event::Finished {
                    id,
                    elapsed: self.jobs[id].elapsed(),
                    result: Err("worker exited without reporting a result".into()),
                });
            }
        }
        self.live.take();
        let counts = self.counts();
        self.message(
            "Summary",
            &format!(
                "{} generated · {} reused · {} failed · {}",
                counts.done,
                counts.reused,
                counts.failed,
                duration(self.started.elapsed())
            ),
            if counts.failed > 0 {
                Level::Error
            } else {
                Level::Success
            },
        );
        self.message("Output", &output.display().to_string(), Level::Info);
        counts.failed
    }
}

fn duration(value: Duration) -> String {
    let secs = value.as_secs();
    if secs >= 3600 {
        format!("{}h {:02}m", secs / 3600, secs % 3600 / 60)
    } else if secs >= 60 {
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else {
        format!("{:.1}s", value.as_secs_f64())
    }
}

#[cfg(test)]
mod tests;
