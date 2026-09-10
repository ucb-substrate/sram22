# SKY130 inputs

| Path | Contents | Maintenance |
| --- | --- | --- |
| `gds/` | Project-maintained cell layouts | Edit with the corresponding cell implementation |
| `spice/` | Project-maintained cell circuits | Keep their interfaces consistent with the layouts |
| `pdk/` | Upstream standard cells and device models, with source commits and licenses | Refresh with `pdk/update.py`; see its [README](pdk/README.md) |

These inputs live together because they describe the same process. The upstream
snapshot has its own directory because it has a separate update source and license,
and `SKY130_OPEN_PDK_ROOT` selects an alternative to that snapshot.

[`src/tech/sky130.rs`](../../src/tech/sky130.rs) owns process constants, custom cell
paths, and PDK selection. [`src/assets.rs`](../../src/assets.rs) extracts the runtime
inputs; [`build.rs`](../../build.rs) embeds custom cells and templates. Hard-macro
bindings live in [`src/blocks/macros`](../../src/blocks/macros/mod.rs).

The executable-level checks live beside the CLI in
[`src/cli/tests.rs`](../../src/cli/tests.rs). Cargo registers them as the
`portable_outputs` integration-test target so they can invoke the built binary:

```bash
cargo test --locked --release --test portable_outputs
```
