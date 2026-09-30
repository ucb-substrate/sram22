use super::*;

/// A permanent message line as the reporter prints it.
fn message(action: &str, text: &str) -> String {
    format!("{action:>ACTION_WIDTH$} {text}")
}

fn reporter(count: usize) -> Reporter {
    let tasks = HashSet::from([TaskKey::GenerateLib]);
    let jobs = (0..count)
        .map(|id| {
            Job::new(
                format!("sram22_{}x32m4w8", 64 << (id % 6)),
                PathBuf::from("/tmp/build"),
                &tasks,
                false,
            )
        })
        .collect();
    Reporter::new(
        jobs,
        Options {
            progress: ProgressMode::Off,
            color: false,
            verbose: false,
            quiet: true,
            live: false,
        },
    )
}

#[test]
fn interleaved_jobs_keep_independent_stages_and_corner_counts() {
    let mut r = reporter(3);
    for id in [0, 1] {
        r.handle(Event::Started {
            id,
            at: Instant::now(),
        });
    }
    r.handle(Event::StageStarted {
        id: 0,
        key: TaskKey::GenerateNetlist,
    });
    r.handle(Event::StageStarted {
        id: 1,
        key: TaskKey::GenerateLib,
    });
    r.handle(Event::CornerFinished {
        id: 1,
        corner: "tt".into(),
    });
    r.handle(Event::CornerFinished {
        id: 1,
        corner: "tt".into(),
    });
    r.handle(Event::StageFinished {
        id: 0,
        key: TaskKey::GenerateNetlist,
    });
    r.handle(Event::StageStarted {
        id: 0,
        key: TaskKey::GenerateLayout,
    });
    assert_eq!(
        r.jobs[0].stage(TaskKey::GenerateNetlist),
        Some(StageState::Done)
    );
    assert_eq!(
        r.jobs[0].stage(TaskKey::GenerateLayout),
        Some(StageState::Running)
    );
    assert_eq!(
        r.jobs[1].stage(TaskKey::GenerateLib),
        Some(StageState::Running)
    );
    assert_eq!(r.jobs[1].corners.len(), 1);
    assert_eq!(r.jobs[2].status, JobStatus::Queued);
    let table = r.render_at(120, 24, Duration::ZERO);
    assert!(table.contains("⠋ 1/3"));
    let narrow = r.render_at(40, 24, Duration::from_millis(100));
    assert!(narrow.contains("⠙ GDS") && narrow.contains("⠙ 1/3 LIB"));
    assert!(table.contains("queued"));
    for name in ["SPICE", "GDS", "Verilog", "LEF", "LIB"] {
        assert!(table.contains(name));
    }
}

#[test]
fn failure_blocks_remaining_collateral_without_changing_other_jobs() {
    let mut r = reporter(2);
    r.handle(Event::Started {
        id: 0,
        at: Instant::now(),
    });
    r.handle(Event::StageFinished {
        id: 0,
        key: TaskKey::GenerateNetlist,
    });
    r.handle(Event::StageStarted {
        id: 0,
        key: TaskKey::GenerateLayout,
    });
    r.handle(Event::Finished {
        id: 0,
        elapsed: Duration::from_secs(2),
        result: Err("GDS: layout failed".into()),
    });
    assert_eq!(
        r.jobs[0].stage(TaskKey::GenerateNetlist),
        Some(StageState::Done)
    );
    assert_eq!(
        r.jobs[0].stage(TaskKey::GenerateLayout),
        Some(StageState::Failed)
    );
    assert_eq!(
        r.jobs[0].stage(TaskKey::GenerateLib),
        Some(StageState::Blocked)
    );
    assert_eq!(
        r.jobs[1].stage(TaskKey::GenerateLib),
        Some(StageState::Pending)
    );
    assert!(r.render(120, 24).contains('✗'));
    assert_eq!(r.counts().failed, 1);
}

#[test]
fn resized_tables_fit_terminal_and_preserve_all_collateral_when_wrapped() {
    let mut r = reporter(2);
    r.jobs[0].name = "sram22_18446744073709551616x128m8w8".into();
    for (width, height) in [
        (160, 30),
        (80, 24),
        (40, 24),
        (20, 10),
        (10, 5),
        (1, 1),
        (0, 0),
    ] {
        let frame = r.render(width, height);
        assert!(frame.lines().count() <= height.saturating_sub(1));
        for line in frame.lines() {
            assert!(
                measure_text_width(line) <= width.saturating_sub(1),
                "{width}x{height}: {line:?}"
            );
        }
        if height >= 24 {
            for name in ["SPICE", "GDS", "Verilog", "LEF", "LIB"] {
                assert!(frame.contains(name), "{width}: {frame}");
            }
        }
    }
}

#[test]
fn large_batches_prioritize_running_rows_and_account_for_hidden_work() {
    let mut r = reporter(100);
    r.jobs[99].name = "active_last_job".into();
    r.handle(Event::Started {
        id: 99,
        at: Instant::now(),
    });
    let frame = r.render(100, 10);
    assert!(frame.contains("active_last_job"));
    assert!(frame.contains("0 running,") && frame.contains("queued not shown"));
    assert!(frame.lines().count() <= 9);
    for id in 0..99 {
        r.handle(Event::Started {
            id,
            at: Instant::now(),
        });
    }
    let frame = r.render(100, 10);
    assert!(frame.contains("0 queued not shown"));
    assert!(frame.contains("100 running"));
}

#[test]
fn reused_and_unreported_jobs_are_finalized_truthfully() {
    let mut r = reporter(2);
    r.jobs[0].status = JobStatus::Reused;
    assert_eq!(r.finish(std::path::Path::new("/tmp/build")), 1);
    assert_eq!(r.jobs[0].status, JobStatus::Reused);
    assert_eq!(r.jobs[1].status, JobStatus::Failed);
    assert!(r.jobs[1]
        .stages
        .iter()
        .all(|(_, state)| *state == StageState::Blocked));
}

#[cfg(feature = "commercial")]
#[test]
fn optional_columns_distinguish_not_requested_from_pending() {
    let tasks = HashSet::from([TaskKey::All, TaskKey::GenerateLib]);
    let mut r = reporter(1);
    r.jobs.push(Job::new(
        "signoff_job".into(),
        PathBuf::new(),
        &tasks,
        false,
    ));
    let frame = r.render(160, 24);
    assert!(frame.contains("DRC") && frame.contains("LVS"));
    assert!(!frame.contains("PEX"));
    assert_eq!(r.cell(&r.jobs[0], TaskKey::RunDrc, "⠋").0, "—");
    assert_eq!(r.cell(&r.jobs[1], TaskKey::RunDrc, "⠋").0, "·");
}

// Exercise indicatif's cursor operations, not just the table's text layout.
#[derive(Clone, Debug, Default)]
struct TestTerminal(std::sync::Arc<std::sync::Mutex<Screen>>);

#[derive(Debug, Default)]
struct Screen {
    lines: Vec<Vec<char>>,
    row: usize,
    col: usize,
}

impl TestTerminal {
    fn contents(&self) -> String {
        self.0
            .lock()
            .unwrap()
            .lines
            .iter()
            .map(|line| line.iter().collect::<String>().trim_end().to_owned())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl indicatif::TermLike for TestTerminal {
    fn width(&self) -> u16 {
        120
    }
    fn height(&self) -> u16 {
        24
    }
    fn move_cursor_up(&self, n: usize) -> io::Result<()> {
        let mut screen = self.0.lock().unwrap();
        screen.row = screen.row.saturating_sub(n);
        Ok(())
    }
    fn move_cursor_down(&self, n: usize) -> io::Result<()> {
        self.0.lock().unwrap().row += n;
        Ok(())
    }
    fn move_cursor_right(&self, n: usize) -> io::Result<()> {
        self.0.lock().unwrap().col += n;
        Ok(())
    }
    fn move_cursor_left(&self, n: usize) -> io::Result<()> {
        let mut screen = self.0.lock().unwrap();
        screen.col = screen.col.saturating_sub(n);
        Ok(())
    }
    fn write_line(&self, s: &str) -> io::Result<()> {
        self.write_str(&format!("{s}\n"))
    }
    fn write_str(&self, s: &str) -> io::Result<()> {
        let mut screen = self.0.lock().unwrap();
        for ch in s.chars() {
            match ch {
                '\n' => {
                    screen.row += 1;
                    screen.col = 0;
                }
                '\r' => screen.col = 0,
                ch => {
                    // indicatif pads permanent messages to the terminal width
                    // and relies on the next printable character to wrap.
                    if screen.col >= self.width() as usize {
                        screen.row += 1;
                        screen.col = 0;
                    }
                    let (row, col) = (screen.row, screen.col);
                    while screen.lines.len() <= row {
                        screen.lines.push(Vec::new());
                    }
                    let line = &mut screen.lines[row];
                    if line.len() <= col {
                        line.resize(col + 1, ' ');
                    }
                    line[col] = ch;
                    screen.col += 1;
                }
            }
        }
        Ok(())
    }
    fn clear_line(&self) -> io::Result<()> {
        let mut screen = self.0.lock().unwrap();
        let row = screen.row;
        if let Some(line) = screen.lines.get_mut(row) {
            line.clear();
        }
        screen.col = 0;
        Ok(())
    }
    fn flush(&self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn live_updates_replace_the_table_and_leave_only_permanent_messages() {
    let terminal = TestTerminal::default();
    let mp = MultiProgress::with_draw_target(indicatif::ProgressDrawTarget::term_like(Box::new(
        terminal.clone(),
    )));
    let table = mp.add(ProgressBar::new_spinner());
    table.set_style(ProgressStyle::with_template("{msg}").unwrap());
    let mut r = reporter(1);
    r.options.quiet = false;
    r.options.progress = ProgressMode::Auto;
    r.live = Some(LiveDisplay { mp, table });
    r.announce(
        std::path::Path::new("sram22.toml"),
        std::path::Path::new("build"),
        1,
    );
    let header = terminal.contents();
    assert!(
        header.starts_with(&format!(
            "     SRAM22 v{} · 1 SRAM · 1 worker\n     Config sram22.toml\n     Output build\n",
            env!("CARGO_PKG_VERSION")
        )),
        "{header:?}"
    );
    r.handle(Event::Started {
        id: 0,
        at: Instant::now(),
    });
    for key in stages(&HashSet::from([TaskKey::GenerateLib])) {
        r.handle(Event::StageStarted { id: 0, key });
        for (tick, spinner) in SPINNER_FRAMES.iter().take(2).enumerate() {
            r.live.as_ref().unwrap().table.set_message(r.render_at(
                120,
                24,
                Duration::from_millis(tick as u64 * 100),
            ));
            let contents = terminal.contents();
            assert_eq!(contents.matches("Building").count(), 1, "{contents}");
            let row = contents
                .lines()
                .find(|line| line.starts_with(&r.jobs[0].name))
                .unwrap();
            assert!(row.contains(spinner), "{contents}");
        }
        r.handle(Event::StageFinished { id: 0, key });
    }
    r.handle(Event::Finished {
        id: 0,
        elapsed: Duration::from_secs(3),
        result: Ok(()),
    });
    r.live
        .as_ref()
        .unwrap()
        .table
        .set_message(r.render(120, 24));
    let contents = terminal.contents();
    assert_eq!(contents.matches("Building").count(), 1, "{contents}");
    assert_eq!(
        contents.matches(&*message("Finished", "sram22_")).count(),
        1,
        "{contents}"
    );
    r.live.take();
    let contents = terminal.contents();
    assert!(
        !contents.contains("Building") && !contents.contains("SPICE"),
        "{contents}"
    );
    assert_eq!(
        contents.matches(&*message("Finished", "sram22_")).count(),
        1,
        "{contents}"
    );
}

#[test]
fn legend_omits_the_spinner_and_marks_unused_stages_only_when_columns_differ() {
    let r = reporter(2);
    let frame = r.render(120, 24);
    let lines: Vec<_> = frame.lines().collect();
    assert_eq!(
        lines[lines.len() - 1],
        "✓ done  · pending  ✗ failed  ! skipped"
    );
    assert_eq!(lines[lines.len() - 2], "", "{frame}");
    assert!(!frame.contains("running  "), "{frame}");
    let short = r.render(120, 6);
    assert!(!short.contains("done"), "{short}");
    assert!(short.contains(&r.jobs[1].name), "{short}");
}

#[test]
fn successful_jobs_never_keep_a_spinner() {
    let mut r = reporter(1);
    r.handle(Event::Started {
        id: 0,
        at: Instant::now(),
    });
    r.handle(Event::StageStarted {
        id: 0,
        key: TaskKey::GenerateLib,
    });
    r.handle(Event::Finished {
        id: 0,
        elapsed: Duration::from_secs(1),
        result: Ok(()),
    });
    assert_eq!(
        r.jobs[0].stage(TaskKey::GenerateLib),
        Some(StageState::Done)
    );
    assert_eq!(
        r.jobs[0].stage(TaskKey::GenerateNetlist),
        Some(StageState::Pending)
    );
}

#[test]
fn durations_scale_from_tenths_to_hours() {
    assert_eq!(duration(Duration::from_millis(1234)), "1.2s");
    assert_eq!(duration(Duration::from_secs(65)), "1m 05s");
    assert_eq!(duration(Duration::from_secs(3725)), "1h 02m");
}

#[test]
fn failure_details_align_under_the_message_text() {
    let terminal = TestTerminal::default();
    let mp = MultiProgress::with_draw_target(indicatif::ProgressDrawTarget::term_like(Box::new(
        terminal.clone(),
    )));
    let table = mp.add(ProgressBar::new_spinner());
    table.set_style(ProgressStyle::with_template("{msg}").unwrap());
    let mut r = reporter(1);
    r.options.quiet = false;
    r.options.progress = ProgressMode::Auto;
    r.live = Some(LiveDisplay { mp, table });
    r.handle(Event::Started {
        id: 0,
        at: Instant::now(),
    });
    r.handle(Event::StageFinished {
        id: 0,
        key: TaskKey::GenerateNetlist,
    });
    r.handle(Event::StageFailed {
        id: 0,
        key: TaskKey::GenerateLayout,
    });
    r.handle(Event::Finished {
        id: 0,
        elapsed: Duration::from_secs(1),
        result: Err("GDS: layout failed\nsecond line".into()),
    });
    let contents = terminal.contents();
    let lines: Vec<_> = contents.lines().collect();
    let first = lines
        .iter()
        .position(|line| line.contains(&message("Failed", "sram22_")))
        .unwrap();
    let column = lines[first].find("sram22_");
    assert_eq!(
        lines[first + 1].find("GDS: layout failed"),
        column,
        "{contents}"
    );
    assert_eq!(lines[first + 2].find("second line"), column, "{contents}");
    assert_eq!(
        lines[first + 3].find("Completed: SPICE"),
        column,
        "{contents}"
    );
    assert_eq!(
        lines[first + 4].find("Work directory:"),
        column,
        "{contents}"
    );
}

#[test]
fn a_failed_stage_keeps_the_job_running_until_it_finishes() {
    let mut r = reporter(1);
    r.handle(Event::Started {
        id: 0,
        at: Instant::now(),
    });
    for key in [TaskKey::GenerateNetlist, TaskKey::GenerateLayout] {
        r.handle(Event::StageStarted { id: 0, key });
    }
    r.handle(Event::StageFinished {
        id: 0,
        key: TaskKey::GenerateNetlist,
    });
    r.handle(Event::StageFailed {
        id: 0,
        key: TaskKey::GenerateLayout,
    });
    assert_eq!(r.jobs[0].status, JobStatus::Running);
    assert_eq!(
        r.jobs[0].stage(TaskKey::GenerateLayout),
        Some(StageState::Failed)
    );
    assert!(r.render(120, 24).contains('✗'));
    r.handle(Event::StageStarted {
        id: 0,
        key: TaskKey::GenerateLib,
    });
    r.handle(Event::StageFinished {
        id: 0,
        key: TaskKey::GenerateLib,
    });
    r.handle(Event::Finished {
        id: 0,
        elapsed: Duration::from_secs(2),
        result: Err("GDS: layout failed".into()),
    });
    assert_eq!(r.jobs[0].status, JobStatus::Failed);
    assert_eq!(
        r.jobs[0].stage(TaskKey::GenerateLib),
        Some(StageState::Done)
    );
    // Never started, so it did not run: shown as skipped.
    assert_eq!(
        r.jobs[0].stage(TaskKey::GenerateVerilog),
        Some(StageState::Blocked)
    );
    assert!(r.render(120, 24).contains("failed"));
}
