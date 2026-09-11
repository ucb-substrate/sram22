#!/usr/bin/env python3
"""Rebuild the bundled SKY130 standard cells from a checked-out open PDK.

Copies standard-cell GDS/SPICE views and their license notices.
Device models, timing tables, and proprietary PDK files are not copied.
"""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile

STRENGTHS = {
    "and2": [0, 1, 2, 4], "and3": [1, 2, 4], "buf": [1, 2, 4, 6, 8, 12, 16],
    "bufbuf": [8, 16], "inv": [1, 2, 4, 6, 8, 16], "tap": [1, 2],
    "mux2": [1, 2, 4, 8], "mux4": [1, 2, 4], "nand2": [1, 2, 4, 8],
    "nand3": [1, 2, 4], "nor2": [1, 2, 4, 8], "nor3": [1, 2, 4],
    "or2": [0, 1, 2, 4], "or3": [1, 2, 4], "xnor2": [1, 2, 4],
    "xnor3": [1, 2, 4], "xor2": [1, 2, 4], "xor3": [1, 2, 4],
    "diode": [2], "dfxtp": [1, 2, 4], "dfrtp": [1, 2, 4], "dfrbp": [1, 2],
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pdk_root", type=Path)
    args = parser.parse_args()
    root = args.pdk_root.resolve()
    out = Path(__file__).resolve().parent
    out.mkdir(parents=True, exist_ok=True)
    files = set()

    for lib in ("sky130_fd_sc_hd", "sky130_fd_sc_hs"):
        for cell, strengths in STRENGTHS.items():
            for strength in strengths:
                for ext in ("gds", "spice"):
                    path = root / f"libraries/{lib}/latest/cells/{cell}/{lib}__{cell}_{strength}.{ext}"
                    # Substrate registers some drive strengths absent upstream.
                    if path.exists():
                        files.add(path.resolve())
    repos = {".": root, **{lib: root / f"libraries/{lib}/latest" for lib in (
        "sky130_fd_sc_hd", "sky130_fd_sc_hs")}}
    sources = {}
    for name, directory in repos.items():
        if subprocess.check_output(["git", "-C", str(directory), "status", "--porcelain"]):
            raise ValueError(f"Refusing to vendor a dirty source checkout: {directory}")
        sources[name] = subprocess.check_output(
            ["git", "-C", str(directory), "rev-parse", "HEAD"], text=True).strip()
        for notice in ("LICENSE", "NOTICE", "AUTHORS", "COPYING"):
            if (directory / notice).is_file():
                files.add((directory / notice).resolve())

    manifest = {"source": "https://github.com/ucb-substrate/skywater-pdk",
                "commits": sources, "files": []}
    archive = io.BytesIO()
    with tarfile.open(fileobj=archive, mode="w", format=tarfile.USTAR_FORMAT) as tar:
        for path in sorted(files):
            name = path.relative_to(root).as_posix()
            data = path.read_bytes()
            info = tarfile.TarInfo(name)
            info.size, info.mode = len(data), 0o644
            tar.addfile(info, io.BytesIO(data))
            manifest["files"].append({"path": name, "bytes": len(data),
                                      "sha256": hashlib.sha256(data).hexdigest()})
    packed = gzip.compress(archive.getvalue(), compresslevel=9, mtime=0)
    (out / "sky130.tar.gz").write_bytes(packed)
    manifest["archive_sha256"] = hashlib.sha256(packed).hexdigest()
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (out / "LICENSE").write_bytes((root / "LICENSE").read_bytes())
    print(f"Vendored {len(files)} files: {len(packed):,} compressed bytes")


if __name__ == "__main__":
    main()
