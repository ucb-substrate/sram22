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
        command
            .env_remove("SKY130_OPEN_PDK_ROOT")
            .env_remove("SKY130_COMMERCIAL_PDK_ROOT");
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
