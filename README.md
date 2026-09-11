# SRAM22

SRAM22 is a configurable single-port SRAM generator for SKY130, developed at UC
Berkeley. It is a research tool; validate generated macros in your integration flow.

## Installation

Install with a current stable Rust toolchain and Cargo:

```bash
cargo install --git https://github.com/ucb-substrate/sram22 --locked sram22
```

## Generate a macro

Save this as `sram22.toml`:

```toml
[[sram]]
num_words = 64
data_width = 32
mux_ratio = 4
write_size = 8
# Optional, with commercial features: pex_level = "rcc"
```

Then run:

```bash
sram22
```

SRAM22 writes GDS, LEF, SPICE, behavioral Verilog, and Liberty timing files to
`build/sram22_64x32m4w8/` beside the configuration file. Add more `[[sram]]` blocks
to generate a batch. Each macro gets its own directory under `build/`, or under
the directory selected with `--output-dir <path>`. A single configuration without
the `[[sram]]` header is also accepted.

```text
-c, --config <CONFIG>          TOML file (default: sram22.toml)
-o, --output-dir <OUTPUT_DIR>  Output directory
-p, --parallel <PARALLEL>      Maximum concurrent macros (default: no limit)
    --spice-corner <CORNER>    SPICE model corner: tt, ss, ff (default: tt)
-h, --help                     Show available options
-V, --version                  Show version
```

With the BWRC manifest, `--liberate` selects Liberate MX characterization;
`--drc` and `--lvs` run Calibre verification. Setting `pex_level` to `r`, `c`, `rc`,
or `rcc` runs extraction for that macro; it is ignored without commercial features.
`--all` enables DRC and LVS but retains interpolated timing unless `--liberate`
is also supplied. Liberate uses the extracted netlist when `pex_level` is set.

### Liberty timing

By default, SRAM22 generates TT, SS, and FF Liberty files from an interpolation
model. Its timing data supports `write_size = 8`, word widths from 8 to 128 bits,
and depths of 64, 128, 256, 512, 1024, or 2048 words. Both mux ratios are supported
except 64 words with mux ratio 8, which has too few rows. Other configurations
require additional timing data or a BWRC build with `--liberate`.
See [the interpolation model](timingdata/INTERPOLATION.md) for its assumptions.
Liberty filenames include a corner suffix, such as
`sram22_64x32m4w8_tt_025C_1v80.lib`, with SS `ss_100C_1v60` and FF `ff_n40C_1v95`.

### SPICE simulation

Include the generated `.spice` file in an ngspice testbench and provide power
supplies, stimuli, and a simulation temperature. Use `--spice-corner tt`, `ss`, or
`ff` to select the device models. When combining several macros in one testbench,
keep one copy of their shared model definitions.

## Configuration

Rows = `num_words / mux_ratio`; columns = `data_width * mux_ratio`.
Valid configurations require positive `num_words`, `data_width`, and `write_size`,
a power-of-two `num_words`, a mux ratio of 4 or 8, data width divisible by write
size, at least 16 rows, and at least 16 columns. Address width is `log2(num_words)`;
write-mask width is `data_width / write_size`.

With `write_size == data_width`, `wmask` is a scalar whole-word write mask. Drive
it high to allow writes.

See the [documentation](https://sram22.com/docs/) for the interface and integration
outlines, and the [published macro catalog](https://github.com/ucb-substrate/sram22_sky130_macros)
for existing layouts and timing libraries. Published GDS files are compressed;
run the catalog's `unzip.sh` or decompress individual `.gds.gz` files.

## Advanced setup

### Local checkout or custom fork

Use a checkout to modify SRAM22 or install a fork. Substitute your fork's URL:

```bash
git clone https://github.com/ucb-substrate/sram22.git
cd sram22
cargo install --path . --locked
```

### BWRC installation

Commercial characterization and Calibre verification require the licensed tools,
commercial PDK, and access to the BWRC Git repositories. Configure SSH access and
add this to `~/.cargo/config.toml`:

```toml
[net]
git-fetch-with-cli = true
```

Set `SKY130_COMMERCIAL_PDK_ROOT` to the commercial PDK root before building, then
select the commercial manifest:

```bash
git clone https://github.com/ucb-substrate/sram22.git
cd sram22
cp Cargo.bwrc.toml Cargo.toml
make install
```

### External open PDK override

To use a custom version of the [open SKY130 PDK](https://github.com/ucb-substrate/skywater-pdk),
set `SKY130_OPEN_PDK_ROOT` when running SRAM22:

```bash
SKY130_OPEN_PDK_ROOT=/absolute/path/to/skywater-pdk sram22
```

Use the linked repository's layout, with the `sky130_fd_sc_hs` and `sky130_fd_pr`
submodules initialized under `libraries/`. The override selects standard-cell views
and device models; SRAM22's custom cells retain their definitions. Missing files
produce an error.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be licensed under the BSD 3-Clause license,
without any additional terms or conditions. Vendored third-party files retain their
own notices and licenses; see [tech/sky130/pdk](tech/sky130/pdk/README.md) for provenance.
