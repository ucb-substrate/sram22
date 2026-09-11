# SKY130 standard cells

`sky130.tar.gz` contains the GDS/SPICE views of the public SKY130 HD and HS
standard cells registered by SRAM22's pinned Substrate adapter. These views always
come from the archive. Device models are external simulation inputs and are excluded
from this snapshot.

The deterministic archive contains 284 files and is approximately 264 KiB compressed.

## Provenance and licenses

The source repository is [ucb-substrate/skywater-pdk](https://github.com/ucb-substrate/skywater-pdk).
`manifest.json` records the exact root and submodule commits, every vendored file's
SHA-256 and size, and the archive SHA-256. Root and library LICENSE/AUTHORS/NOTICE
files, where present, are preserved inside the archive; the root Apache-2.0 license
is also provided alongside it. File-level copyright and license notices are retained.
Vendored files retain their upstream licenses, independently of SRAM22's BSD license.

To reproduce the snapshot from a clean checkout with initialized HD/HS submodules:

```bash
python3 tech/sky130/pdk/update.py /path/to/skywater-pdk
```

The script refuses dirty source checkouts. Review commit/hash changes and rerun
portable-output tests when updating the snapshot or the Substrate cell registry:

```bash
cargo test --locked --release --test portable_outputs
```
