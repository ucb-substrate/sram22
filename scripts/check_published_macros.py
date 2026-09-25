#!/usr/bin/env python3
"""Check that sram22 still generates the published sky130 macros unchanged.

    check_published_macros.py --sram22 BIN --catalog DIR --out DIR [--jobs N] [--only NAME ...]
    check_published_macros.py --self-test --sram22 BIN --catalog DIR --out DIR

Every `sram22_<words>x<width>m<mux>w<wmask>` directory in the catalog
(github.com/ucb-substrate/sram22_sky130_macros) is regenerated and compared view by view:

GDS      flattened per-layer XOR of the top cell, plus the multiset of text labels
         (layer, string, position). Subcell names and hierarchy are ignored: sram22
         numbers its uniquified cells differently from run to run.
LEF      header lines in order; RECT/POLYGON lines as a multiset within each block,
         since pin shapes are written in a run-dependent order.
Verilog  line by line, ignoring whitespace.
SPICE    the top subcircuit's ports, and a census of flattened transistor fingers by
         (model, W per finger, L). The catalog was written by an older netlister, so
         the text cannot match exactly.
Liberty  not checked; it comes from a separate characterization flow. Open-source
         builds interpolate it from timingdata/, which covers only some shapes; a
         missing table is reported as a note. Any other generator error fails.

--self-test regenerates one macro, then checks that the comparisons flag a deleted
shape, a 1 nm shift, a renamed label, a moved LEF rectangle, a renamed Verilog port and
a resized transistor. Exit status is non-zero if any macro differs or any mutation
goes undetected.
"""
import argparse
import collections
import gzip
import os
import re
import shutil
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

import klayout.db as db

NAME_RE = re.compile(r"^sram22_(\d+)x(\d+)m(\d+)w(\d+)$")
GEOM_RE = re.compile(r"^\s*(RECT|POLYGON)\b")


# GDS


def gds_views(path, top_name):
    """Merged geometry per (layer, datatype), and flattened text labels."""
    ly = db.Layout()
    ly.read(path)
    top = ly.cell(top_name)
    if top is None:
        raise ValueError(f"{path}: no cell named {top_name}")
    regions, labels = {}, collections.Counter()
    for li in ly.layer_indexes():
        info = ly.get_info(li)
        key = (info.layer, info.datatype)
        region = db.Region(top.begin_shapes_rec(li))
        if not region.is_empty():
            regions[key] = region.merged()
        it = top.begin_shapes_rec(li)
        it.shape_flags = db.Shapes.STexts
        while not it.at_end():
            text = it.shape().text.transformed(it.trans())
            labels[(key, text.string, text.x, text.y)] += 1
            it.next()
    return regions, labels


def compare_gds(new, ref, top):
    problems = []
    ra, la = gds_views(new, top)
    rb, lb = gds_views(ref, top)
    for key in sorted(set(ra) | set(rb)):
        a, b = ra.get(key, db.Region()), rb.get(key, db.Region())
        x = a ^ b
        if not x.is_empty():
            problems.append(f"GDS layer {key[0]}/{key[1]}: {x.count()} differing polygons")
    if la != lb:
        extra, missing = la - lb, lb - la
        sample = next(iter(extra or missing))
        problems.append(f"GDS labels: {sum(extra.values())} new, {sum(missing.values())} "
                        f"missing (e.g. {sample[1]!r} on {sample[0][0]}/{sample[0][1]})")
    return problems


# LEF, Verilog


def lef_canon(path):
    out, block = [], []
    for line in open(path):
        if GEOM_RE.match(line):
            block.append(line.strip())
            continue
        if block:
            out.append(("geom", tuple(sorted(block))))
            block = []
        out.append(("line", line.strip()))
    if block:
        out.append(("geom", tuple(sorted(block))))
    return out


def verilog_canon(path):
    return [" ".join(line.split()) for line in open(path) if line.strip()]


# SPICE

SUFFIX = {"f": 1e-15, "p": 1e-12, "n": 1e-9, "u": 1e-6, "m": 1e-3, "k": 1e3,
          "meg": 1e6, "g": 1e9, "t": 1e12}


def spice_num(s):
    m = re.fullmatch(r"([-+]?[0-9.]+(?:e[-+]?\d+)?)(meg|[fpnumkgt])?", s.lower())
    if not m:
        raise ValueError(s)
    return float(m.group(1)) * SUFFIX.get(m.group(2) or "", 1.0)


def spice_parse(path):
    """{subckt: (ports, [(target, nets, params)])}, names lowercased."""
    lines = []
    for raw in open(path):
        raw = raw.rstrip("\n")
        if raw.startswith("+") and lines:
            lines[-1] += " " + raw[1:]
        elif raw.strip() and not raw.lstrip().startswith("*"):
            lines.append(raw.strip())
    subckts, cur = {}, None
    for line in lines:
        tok = line.split()
        head = tok[0].lower()
        if head == ".subckt":
            cur = tok[1].lower()
            subckts[cur] = ([t.lower() for t in tok[2:] if "=" not in t], [])
        elif head == ".ends":
            cur = None
        elif cur and head.startswith("x"):
            pos = [t for t in tok[1:] if "=" not in t]
            params = dict(t.lower().split("=", 1) for t in tok[1:] if "=" in t)
            subckts[cur][1].append((pos[-1].lower(), pos[:-1], params))
    return subckts


def device_census(subckts, top):
    """Flatten `top`, counting transistor fingers by (model, W per finger, L)."""
    memo = {}

    def count(name):
        if name in memo:
            return memo[name]
        c = collections.Counter()
        for target, _, params in subckts[name][1]:
            mult = int(round(spice_num(params.get("m", "1")) * spice_num(params.get("mult", "1"))))
            if target in subckts:
                for k, v in count(target).items():
                    c[k] += v * mult
            else:
                # The old netlister writes a multi-finger device once with nf=N; the
                # new one writes N single-finger devices. Both give sizes in um.
                w = spice_num(params.get("w", "0"))
                l = spice_num(params.get("l", "0"))
                c[(target, round(w, 4), round(l, 4))] += mult * int(params.get("nf", "1"))
        memo[name] = c
        return c

    return count(top)


def compare_spice(new, ref, top):
    problems = []
    a, b = spice_parse(new), spice_parse(ref)
    top = top.lower()
    if a[top][0] != b[top][0]:
        problems.append("SPICE top-level ports differ")
    ca, cb = device_census(a, top), device_census(b, top)
    if ca != cb:
        diff = (ca - cb) + (cb - ca)
        problems.append(f"SPICE transistor census differs ({sum(diff.values())} fingers)")
    return problems


# Driver


def compare(new_prefix, ref_prefix, name):
    """Compare regenerated views at `new_prefix`.* with the catalog's at `ref_prefix`.*."""
    problems = compare_gds(new_prefix + ".gds", ref_prefix + ".gds.gz", name)
    if lef_canon(new_prefix + ".lef") != lef_canon(ref_prefix + ".lef"):
        problems.append("LEF differs")
    if verilog_canon(new_prefix + ".v") != verilog_canon(ref_prefix + ".v"):
        problems.append("Verilog differs")
    problems += compare_spice(new_prefix + ".spice", ref_prefix + ".spice", name)
    return problems


def generate(sram22, name, out_dir):
    words, width, mux, wmask = map(int, NAME_RE.match(name).groups())
    d = os.path.join(out_dir, name)
    os.makedirs(d, exist_ok=True)
    cfg = os.path.join(d, "sram22.toml")
    with open(cfg, "w") as f:
        f.write(f"[[sram]]\nnum_words = {words}\ndata_width = {width}\n"
                f"mux_ratio = {mux}\nwrite_size = {wmask}\n")
    start = time.time()
    p = subprocess.run([sram22, "-c", cfg, "-o", os.path.join(d, "out")],
                       capture_output=True, text=True, cwd=d)
    with open(os.path.join(d, "gen.log"), "w") as f:
        f.write(p.stdout + p.stderr)
    prefix = os.path.join(d, "out", name, name)
    secs = time.time() - start
    missing = [ext for ext in (".gds", ".lef", ".v", ".spice") if not os.path.exists(prefix + ext)]
    if missing:
        return None, f"generation failed, no {', '.join(missing)} (see gen.log)", secs
    if p.returncode != 0:
        # Liberty is written last and is not compared. Open-source builds only have
        # timing tables for some shapes (timingdata/), so a missing table is noted,
        # not failed; any other error is a failure.
        if "no timing data for" not in p.stderr + p.stdout:
            return None, f"sram22 exited with status {p.returncode} (see gen.log)", secs
        return prefix, "no .lib written (no timing data for this shape)", secs
    return prefix, None, secs


def check_one(args, name):
    prefix, note, secs = generate(args.sram22, name, args.out)
    if prefix is None:
        return name, [note], None, secs
    return name, compare(prefix, os.path.join(args.catalog, name, name), name), note, secs


def macros(catalog, only):
    names = sorted(d for d in os.listdir(catalog) if NAME_RE.match(d))
    if only:
        missing = set(only) - set(names)
        if missing:
            raise SystemExit(f"not in catalog: {', '.join(sorted(missing))}")
        names = [n for n in names if n in only]
    if not names:
        raise SystemExit(f"no sram22_* macros found in {catalog}")
    return names


def run(args):
    names = macros(args.catalog, args.only)
    failed = 0
    with ThreadPoolExecutor(max_workers=args.jobs) as ex:
        for name, problems, note, secs in ex.map(lambda n: check_one(args, n), names):
            status = "ok" if not problems else "DIFFERS"
            print(f"{name:<24} {status:<8} {secs:6.1f}s", flush=True)
            for p in problems:
                print(f"    {p}", flush=True)
            if note:
                print(f"    note: {note}", flush=True)
            failed += bool(problems)
    print(f"\n{len(names) - failed}/{len(names)} published macros unchanged")
    return 1 if failed else 0


# Self-test: every mutation of a known-good macro must be caught.


def mutate_copy(src_prefix, dst_dir, name, mutation):
    os.makedirs(dst_dir, exist_ok=True)
    dst = os.path.join(dst_dir, name)
    for ext in (".gds", ".lef", ".v", ".spice"):
        shutil.copy(src_prefix + ext, dst + ext)
    if mutation.startswith("gds_"):
        ly = db.Layout()
        ly.read(dst + ".gds")
        top = ly.cell(name)
        done = False
        for li in ly.layer_indexes():
            shapes = top.shapes(li)
            for sh in list(shapes.each()):
                if mutation == "gds_label_rename" and sh.is_text():
                    sh.text_string = sh.text_string + "_x"
                elif mutation == "gds_shift_1nm" and sh.is_box():
                    sh.transform(db.Trans(1, 0))
                elif mutation == "gds_delete_shape" and sh.is_box():
                    # Only a deletion that changes what is drawn counts: a box that is
                    # covered by other shapes on its layer leaves the geometry intact.
                    box = sh.box
                    window = db.Region(box)
                    local = lambda: db.Region(top.begin_shapes_rec_overlapping(li, box)) & window
                    before = local()
                    shapes.erase(sh)
                    if (before ^ local()).is_empty():
                        shapes.insert(box)
                        continue
                else:
                    continue
                done = True
                break
            if done:
                break
        if not done:
            raise RuntimeError(f"{mutation}: no suitable shape in the top cell")
        ly.write(dst + ".gds")
    else:
        path = {"lef_move_rect": ".lef", "verilog_rename_port": ".v",
                "spice_resize_device": ".spice"}[mutation]
        text = open(dst + path).read()
        if mutation == "lef_move_rect":
            new = re.sub(r"(RECT\s+)(-?[0-9.]+)", lambda m: f"{m.group(1)}{float(m.group(2)) + 0.005:.3f}",
                         text, count=1)
        elif mutation == "verilog_rename_port":
            new = re.sub(r"\bclk\b", "clk_x", text, count=1)
        else:
            new = re.sub(r"(\bw=)([0-9.]+)", lambda m: m.group(1) + str(float(m.group(2)) * 2), text,
                         count=1, flags=re.I)
        if new == text:
            raise RuntimeError(f"{mutation}: pattern not found")
        open(dst + path, "w").write(new)
    return dst


MUTATIONS = ["gds_delete_shape", "gds_shift_1nm", "gds_label_rename", "lef_move_rect",
             "verilog_rename_port", "spice_resize_device"]


def self_test(args):
    name = (args.only or ["sram22_64x32m4w8"])[0]
    prefix, note, _ = generate(args.sram22, name, args.out)
    if prefix is None:
        raise SystemExit(f"self-test: {note}")
    ref = os.path.join(args.catalog, name, name)
    base = compare(prefix, ref, name)
    print(f"unmodified {name:<28} {'ok' if not base else 'DIFFERS: ' + '; '.join(base)}")
    missed = bool(base)
    for m in MUTATIONS:
        # Compare the catalog against a mutated copy of the regenerated macro.
        dst = mutate_copy(prefix, os.path.join(args.out, "self-test", m), name, m)
        with gzip.open(dst + ".gds.gz", "wb") as f:
            f.write(open(dst + ".gds", "rb").read())
        problems = compare(prefix, dst, name)
        print(f"mutation {m:<30} {'caught: ' + problems[0] if problems else 'MISSED'}")
        missed |= not problems
    print("\nself-test", "FAILED" if missed else "passed")
    return 1 if missed else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--sram22", required=True, help="path to the sram22 binary")
    ap.add_argument("--catalog", required=True, help="checkout of sram22_sky130_macros")
    ap.add_argument("--out", required=True, help="scratch directory for generated macros")
    ap.add_argument("--jobs", type=int, default=os.cpu_count())
    ap.add_argument("--only", nargs="*", help="check only these macros")
    ap.add_argument("--self-test", action="store_true", help="check that the comparisons catch mutations")
    args = ap.parse_args()
    args.sram22 = os.path.abspath(args.sram22)
    args.out = os.path.abspath(args.out)
    sys.exit(self_test(args) if args.self_test else run(args))


if __name__ == "__main__":
    main()
