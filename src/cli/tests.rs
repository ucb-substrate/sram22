// Executable-level tests, registered as the portable_outputs Cargo test target.
use std::collections::HashSet;
use std::fs;
use std::process::Command;

fn check_gds(bytes: &[u8]) {
    let mut names = HashSet::new();
    let mut references = HashSet::new();
    let mut position = 0;
    while position < bytes.len() {
        let length = u16::from_be_bytes([bytes[position], bytes[position + 1]]) as usize;
        assert!(length >= 4 && position + length <= bytes.len());
        let kind = bytes[position + 2];
        if kind == 6 || kind == 18 {
            // STRNAME or SNAME
            let name = String::from_utf8_lossy(&bytes[position + 4..position + length])
                .trim_end_matches('\0')
                .to_string();
            if kind == 6 {
                names.insert(name);
            } else {
                references.insert(name);
            }
        }
        position += length;
    }
    assert!(!references.is_empty());
    assert!(
        references.is_subset(&names),
        "GDS contains unresolved cell references"
    );
}

#[test]
fn generated_outputs_are_portable_for_both_mux_ratios() {
    let run = tempfile::tempdir().unwrap();
    for (words, width, mux, write_size) in [(64, 8, 4, 8), (128, 16, 8, 8)] {
        let config = run.path().join("sram22.toml");
        fs::write(
            &config,
            format!(
                "num_words={words}\ndata_width={width}\nmux_ratio={mux}\nwrite_size={write_size}\n"
            ),
        )
        .unwrap();
        let output = run.path().join(format!("m{mux}"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_sram22"));
        command.env_remove("SKY130_OPEN_PDK_ROOT").env(
            "SKY130_COMMERCIAL_PDK_ROOT",
            run.path().join(format!("absent-commercial-pdk-{mux}")),
        );
        if mux == 8 {
            // External PDKs and simulation settings cannot select generation inputs.
            command.env("SKY130_OPEN_PDK_ROOT", run.path().join("absent-pdk"));
        }
        let result = command
            .args([
                "--config",
                config.to_str().unwrap(),
                "--output-dir",
                output.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let name = format!("sram22_{words}x{width}m{mux}w{write_size}");
        let output = output.join(&name);
        let spice = fs::read_to_string(output.join(format!("{name}.spice"))).unwrap();
        for line in spice.lines() {
            let token = line
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            assert!(
                !matches!(token.as_str(), ".inc" | ".include" | ".lib" | ".model"),
                "external reference or device model in circuit export: {line}"
            );
        }
        assert!(spice.contains("sky130_fd_pr__nfet_01v8"));
        assert!(spice.contains(".subckt sky130_fd_sc_hs__"));
        assert!(!spice.contains("sram22-assets-") && !spice.contains(env!("CARGO_MANIFEST_DIR")));
        let isolated = run.path().join(format!("isolated-{mux}.gds"));
        fs::copy(output.join(format!("{name}.gds")), &isolated).unwrap();
        check_gds(&fs::read(isolated).unwrap());
        let verilog = fs::read_to_string(output.join(format!("{name}.v"))).unwrap();
        let lef = fs::read_to_string(output.join(format!("{name}.lef"))).unwrap();
        for suffix in ["tt_025C_1v80", "ss_100C_1v60", "ff_n40C_1v95"] {
            let liberty = fs::read_to_string(output.join(format!("{name}_{suffix}.lib"))).unwrap();
            assert!(liberty.contains(&format!("cell ({name})")));
        }
        if write_size == width {
            assert!(verilog.contains("input wmask;") && verilog.contains("if (wmask)"));
            assert!(lef.contains("PIN wmask "));
        } else {
            assert!(
                verilog.contains("input [WMASK_WIDTH-1:0] wmask;")
                    && verilog.contains("if (wmask[1])")
            );
            assert!(lef.contains("PIN wmask[1]"));
        }
    }
}

fn batch_config() -> String {
    [64, 128]
        .iter()
        .map(|words| {
            format!("[[sram]]\nnum_words={words}\ndata_width=8\nmux_ratio=4\nwrite_size=8\n")
        })
        .collect()
}

/// A permanent message line as the CLI prints it: a right-aligned label, then text.
fn message(action: &str, text: &str) -> String {
    format!("{action:>11} {text}")
}

fn cli(config: &std::path::Path, output: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sram22"));
    command.args([
        "--config",
        config.to_str().unwrap(),
        "--output-dir",
        output.to_str().unwrap(),
    ]);
    command.env("NO_COLOR", "1");
    command
}

#[test]
fn batch_generation_reuse_and_partial_failure_have_readable_output() {
    let run = tempfile::tempdir().unwrap();
    let config = run.path().join("batch.toml");
    let output = run.path().join("build");
    fs::write(&config, batch_config()).unwrap();
    fs::create_dir(&output).unwrap();
    // One SRAM fails during setup; another must still complete.
    let blocked = output.join("sram22_64x8m4w8");
    fs::write(&blocked, "not a directory").unwrap();
    let result = cli(&config, &output)
        .args(["--parallel", "2"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    assert!(!stderr.contains('\x1b'));
    assert!(
        stderr.contains(&message("Failed", "sram22_64x8m4w8")) && stderr.contains("Setup:"),
        "{stderr}"
    );
    assert!(
        stderr.contains(&message("Finished", "sram22_128x8m4w8")),
        "{stderr}"
    );
    assert!(
        stderr.contains("1 generated · 0 reused · 1 failed"),
        "{stderr}"
    );
    assert!(stderr.contains("Work directory:"));
    assert!(!stderr.contains("panicked"));

    // Retry generates just the failed SRAM and explicitly reports reuse.
    fs::remove_file(blocked).unwrap();
    let result = cli(&config, &output)
        .args(["--progress", "plain", "--verbose"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success(), "{stderr}");
    assert!(
        stderr.contains(&message("Reused", "sram22_128x8m4w8")),
        "{stderr}"
    );
    assert!(
        stderr.contains("1 generated · 1 reused · 0 failed"),
        "{stderr}"
    );
    for stage in ["SPICE", "GDS", "Verilog", "LEF", "LIB"] {
        assert!(stderr.contains(stage));
    }
    for corner in ["tt_025C_1v80", "ss_100C_1v60", "ff_n40C_1v95"] {
        assert!(stderr.contains(corner));
    }

    let result = cli(&config, &output).output().unwrap();
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success());
    assert_eq!(stderr.matches(&*message("Reused", "sram22_")).count(), 2);
    assert!(stderr.contains("0 generated · 2 reused · 0 failed"));
    assert!(!stderr.contains("Started"));

    let result = cli(&config, &output)
        .args(["--force", "--progress", "plain"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success(), "{stderr}");
    assert_eq!(
        stderr.matches(&*message("Started", "sram22_")).count(),
        2,
        "{stderr}"
    );
    assert!(!stderr.contains("Reused"), "{stderr}");
    assert!(
        stderr.contains("2 generated · 0 reused · 0 failed"),
        "{stderr}"
    );

    let result = cli(&config, &output).arg("--quiet").output().unwrap();
    assert!(result.status.success() && result.stdout.is_empty() && result.stderr.is_empty());
    let result = cli(&config, &output)
        .args(["--progress", "off"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(result.status.success() && stderr.contains("Summary"));
    assert!(!stderr.contains("Reused") && !stderr.contains('\x1b'));
    let result = cli(&config, &output)
        .args(["--color", "always"])
        .output()
        .unwrap();
    assert!(result.status.success() && result.stderr.contains(&0x1b));
}

#[test]
fn invalid_batches_fail_before_creating_outputs() {
    let run = tempfile::tempdir().unwrap();
    let config = run.path().join("batch.toml");
    let output = run.path().join("must-not-exist");
    let single = "[[sram]]\nnum_words=64\ndata_width=8\nmux_ratio=4\nwrite_size=8\n";
    for (contents, expected) in [
        (format!("{single}{single}"), "duplicates entry 1"),
        (
            single.replace("num_words=64", "num_words=4096"),
            "no timing data",
        ),
        (single.replace("mux_ratio=4", "mux_ratio=3"), "mux_ratio"),
        ("sram=[]\n".into(), "must not be empty"),
        (
            single.replace("data_width=8\n", ""),
            "missing field `data_width`",
        ),
    ] {
        fs::write(&config, contents).unwrap();
        let result = cli(&config, &output).output().unwrap();
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(!result.status.success(), "{stderr}");
        assert!(stderr.contains(expected), "wanted {expected:?}: {stderr}");
        assert!(!output.exists());
    }
    fs::write(&config, single).unwrap();
    let result = cli(&config, &output)
        .args(["--parallel", "0"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("--parallel must be greater than zero")
    );
}
