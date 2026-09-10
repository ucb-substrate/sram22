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
    for (words, mux, write_size, corner) in [(64, 4, 8, "tt"), (128, 8, 4, "ss")] {
        let config = run.path().join("sram22.toml");
        fs::write(
            &config,
            format!("num_words={words}\ndata_width=8\nmux_ratio={mux}\nwrite_size={write_size}\n"),
        )
        .unwrap();
        let output = run.path().join(format!("m{mux}"));
        let result = Command::new(env!("CARGO_BIN_EXE_sram22"))
            .env_remove("SKY130_OPEN_PDK_ROOT")
            .env_remove("SKY130_COMMERCIAL_PDK_ROOT")
            .args([
                "--config",
                config.to_str().unwrap(),
                "--output-dir",
                output.to_str().unwrap(),
                "--spice-corner",
                corner,
            ])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let name = format!("sram22_{words}x8m{mux}w{write_size}");
        let spice = fs::read_to_string(output.join(format!("{name}.spice"))).unwrap();
        for line in spice.lines() {
            let token = line
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            assert!(
                !matches!(token.as_str(), ".inc" | ".include" | ".lib"),
                "external SPICE reference: {line}"
            );
        }
        assert!(spice.contains(&format!("device models: {corner} corner")));
        assert!(spice.to_ascii_lowercase().contains(".model"));
        assert!(!spice.contains("sram22-assets-") && !spice.contains(env!("CARGO_MANIFEST_DIR")));
        let isolated = run.path().join(format!("isolated-{mux}.gds"));
        fs::copy(output.join(format!("{name}.gds")), &isolated).unwrap();
        check_gds(&fs::read(isolated).unwrap());
        let verilog = fs::read_to_string(output.join(format!("{name}.v"))).unwrap();
        let lef = fs::read_to_string(output.join(format!("{name}.lef"))).unwrap();
        if write_size == 8 {
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

#[test]
fn explicit_incomplete_pdk_does_not_fall_back_to_bundled() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.toml");
    fs::write(
        &config,
        "num_words=64\ndata_width=8\nmux_ratio=4\nwrite_size=8\n",
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_sram22"))
        .env("SKY130_OPEN_PDK_ROOT", directory.path())
        .args([
            "--config",
            config.to_str().unwrap(),
            "--output-dir",
            directory.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("is missing"));
}
