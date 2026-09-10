# SKY130 source data

`sky130.tar.gz` is a deterministic snapshot of public SKY130 files used by SRAM22
and its pinned Substrate PDK adapter. It includes GDS/SPICE views of the HD and HS
standard cells registered by that adapter, and the complete recursive file closure
of the primitive-device `models/sky130.lib.spice` library.

The archive is approximately 4 MiB compressed (31 MiB of input files).
For alternate runtime sources, see the [external PDK override](../../../README.md#external-open-pdk-override).

## Provenance and licenses

The source repository is [ucb-substrate/skywater-pdk](https://github.com/ucb-substrate/skywater-pdk).
`manifest.json` records the exact root and submodule commits, every vendored file's
SHA-256 and size, and the archive SHA-256. Root and library LICENSE/AUTHORS/NOTICE
files, where present, are preserved inside the archive; the root Apache-2.0 license
is also provided alongside it. File-level copyright and license notices are retained.
Vendored files retain their upstream licenses, independently of SRAM22's BSD license.

To reproduce the snapshot from a clean checkout with initialized HD/HS/PR submodules:

```bash
python3 tech/sky130/pdk/update.py /path/to/skywater-pdk
```

The script refuses dirty source checkouts. Review commit/hash changes and rerun
portable-output tests when updating the snapshot or the Substrate cell registry:

```bash
cargo test --locked --release --test portable_outputs
```
