# SKY130 inputs

| Path | Contents | Maintenance |
| --- | --- | --- |
| `gds/` | Custom and vendored standard-cell layouts | Edit custom cells with their corresponding implementation |
| `spice/` | Custom and vendored standard-cell circuits | Keep custom cell interfaces consistent with their layouts |

Cells are organized by view. Vendored standard cells retain their upstream
`sky130_fd_sc_hd__` and `sky130_fd_sc_hs__` filename prefixes. All layout and circuit
generation uses these bundled inputs.

[`src/tech/sky130.rs`](../../src/tech/sky130.rs) owns process constants, custom cell
paths, and the process adapter. The adapter loads external device models through
`SKY130_OPEN_PDK_ROOT` only when simulating. [`src/assets.rs`](../../src/assets.rs) extracts the runtime
inputs; [`build.rs`](../../build.rs) embeds the cell files and templates. Hard-macro
bindings live in [`src/blocks/macros`](../../src/blocks/macros/mod.rs).

## Standard-cell provenance and licenses

The 14 public SKY130 standard cells come from
[ucb-substrate/skywater-pdk](https://github.com/ucb-substrate/skywater-pdk): two HD
cells for the TDC blocks and twelve HS cells for the SRAM control and column
circuits. Their source revisions are:

| Source | Commit |
| --- | --- |
| `skywater-pdk` | `6fcd983f8be885bd831551a8daad7b4f1657f33e` |
| `sky130_fd_sc_hd` | `cb4c7daccb8633987045e56271ea95fc89c2a034` |
| `sky130_fd_sc_hs` | `8af46747e1cc01daefcdfab09d04a5a3ca405df5` |

Upstream notices live in this directory with `.skywater`,
`.sky130_fd_sc_hd`, or `.sky130_fd_sc_hs` suffixes to identify their source.
File-level copyright and license notices are retained. Vendored files retain
their upstream licenses, independently of SRAM22's BSD license.

When replacing standard-cell files, update the source revisions above and run
the portable-output checks. These checks live beside the CLI in
[`src/cli/tests.rs`](../../src/cli/tests.rs); Cargo registers them as the
`portable_outputs` integration-test target:

```bash
cargo test --locked --release --test portable_outputs
```
