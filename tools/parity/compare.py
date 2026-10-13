"""Compare reference screenshots (original program) with Firepixel screenshots.

For every equally named PNG pair of one parity unit this writes a side-by-side image
(reference | Firepixel | amplified difference) and a metrics row.

Default locations (relative to the repository):
  reference : ..\\re\\_testdateien\\par\\ref\\<unit>\\
  firepixel : docs\\parity\\img\\<unit>\\
  output    : ..\\re\\_testdateien\\par\\cmp\\<unit>\\  (summary.csv, summary.md, <name>.png)

Usage:
  python -I tools/parity/compare.py <unit> [<unit> ...] [--all] [--threshold 16]
         [--ref-root DIR] [--fp-root DIR] [--out-root DIR] [--no-images]

Requires Pillow; uses numpy when available.
"""

from __future__ import annotations

import argparse
import csv
import sys
from pathlib import Path

try:
    from PIL import Image, ImageChops, ImageDraw
except ImportError:  # pragma: no cover
    sys.exit("compare.py needs Pillow (PIL).")

try:
    import numpy as np
except ImportError:  # pragma: no cover
    np = None

REPO = Path(__file__).resolve().parents[2]
WORKSPACE = REPO.parent
DEFAULT_REF = WORKSPACE / "re" / "_testdateien" / "par" / "ref"
DEFAULT_FP = REPO / "docs" / "parity" / "img"
DEFAULT_OUT = WORKSPACE / "re" / "_testdateien" / "par" / "cmp"

GAP = 8
LABEL_H = 18
BG = (255, 0, 255, 255)  # padding colour for size mismatches


def load(path: Path) -> Image.Image:
    with Image.open(path) as im:
        return im.convert("RGBA")


def pad_to(im: Image.Image, w: int, h: int) -> Image.Image:
    if im.size == (w, h):
        return im
    out = Image.new("RGBA", (w, h), BG)
    out.paste(im, (0, 0))
    return out


def metrics(a: Image.Image, b: Image.Image, threshold: int) -> tuple[dict, Image.Image]:
    """a, b have equal size. Returns metrics and an amplified difference image."""
    if np is not None:
        x = np.asarray(a, dtype=np.int16)
        y = np.asarray(b, dtype=np.int16)
        d = np.abs(x - y)
        per_px = d.max(axis=2)
        total = per_px.size
        diff_px = int((per_px > threshold).sum())
        mean = float(d.mean())
        maxd = int(d.max()) if total else 0
        amp = np.clip(per_px.astype(np.int32) * 4, 0, 255).astype(np.uint8)
        heat = np.zeros((*amp.shape, 4), dtype=np.uint8)
        heat[..., 0] = amp
        heat[..., 3] = 255
        heat[per_px > threshold, 1] = 64
        diff_img = Image.fromarray(heat, "RGBA")
    else:
        diff = ImageChops.difference(a, b)
        bands = diff.split()
        per_px = bands[0]
        for band in bands[1:]:
            per_px = ImageChops.lighter(per_px, band)
        px_hist = per_px.histogram()
        total = a.width * a.height
        diff_px = sum(px_hist[threshold + 1:])
        hist = diff.histogram()
        n = sum(hist) or 1
        mean = sum((i % 256) * c for i, c in enumerate(hist)) / n
        maxd = max((v for v, c in enumerate(px_hist) if c), default=0)
        amp = per_px.point(lambda v: min(255, v * 4))
        zero = Image.new("L", a.size, 0)
        mask = per_px.point(lambda v: 64 if v > threshold else 0)
        diff_img = Image.merge("RGBA", (amp, mask, zero, Image.new("L", a.size, 255)))
    share = (diff_px / total * 100.0) if total else 0.0
    return {
        "mean_abs": round(mean, 3),
        "diff_pixels": diff_px,
        "diff_share_pct": round(share, 3),
        "max_abs": maxd,
        "equal": diff_px == 0,
    }, diff_img


def side_by_side(a: Image.Image, b: Image.Image, d: Image.Image) -> Image.Image:
    w, h = a.size
    out = Image.new("RGBA", (w * 3 + GAP * 2, h + LABEL_H), (255, 255, 255, 255))
    draw = ImageDraw.Draw(out)
    for i, (im, label) in enumerate(((a, "original"), (b, "firepixel"), (d, "diff x4"))):
        x = i * (w + GAP)
        draw.text((x + 2, 2), label, fill=(0, 0, 0, 255))
        out.paste(im, (x, LABEL_H))
    return out


def compare_unit(unit: str, ref_root: Path, fp_root: Path, out_root: Path,
                 threshold: int, write_images: bool) -> int:
    ref_dir, fp_dir, out_dir = ref_root / unit, fp_root / unit, out_root / unit
    ref = {p.name: p for p in ref_dir.glob("*.png")} if ref_dir.is_dir() else {}
    fp = {p.name: p for p in fp_dir.glob("*.png")} if fp_dir.is_dir() else {}
    if not ref and not fp:
        print(f"{unit}: no images in {ref_dir} or {fp_dir}")
        return 1
    out_dir.mkdir(parents=True, exist_ok=True)
    rows = []
    for name in sorted(set(ref) | set(fp)):
        row = {"name": name, "ref_size": "", "fp_size": "", "size_match": "",
               "mean_abs": "", "diff_pixels": "", "diff_share_pct": "", "max_abs": "",
               "equal": "", "note": ""}
        if name not in ref:
            row["note"] = "missing reference"
        elif name not in fp:
            row["note"] = "missing firepixel"
        else:
            a, b = load(ref[name]), load(fp[name])
            row["ref_size"] = f"{a.width}x{a.height}"
            row["fp_size"] = f"{b.width}x{b.height}"
            row["size_match"] = a.size == b.size
            w, h = max(a.width, b.width), max(a.height, b.height)
            a, b = pad_to(a, w, h), pad_to(b, w, h)
            m, d = metrics(a, b, threshold)
            row.update(m)
            if not row["size_match"]:
                row["note"] = "size differs, aligned top-left"
            if write_images:
                side_by_side(a, b, d).save(out_dir / name)
        rows.append(row)

    fields = list(rows[0].keys())
    with open(out_dir / "summary.csv", "w", newline="", encoding="utf-8") as f:
        wr = csv.DictWriter(f, fieldnames=fields)
        wr.writeheader()
        wr.writerows(rows)
    with open(out_dir / "summary.md", "w", encoding="utf-8") as f:
        f.write(f"# {unit} – image comparison (threshold {threshold})\n\n")
        f.write("| " + " | ".join(fields) + " |\n")
        f.write("|" + "---|" * len(fields) + "\n")
        for r in rows:
            f.write("| " + " | ".join(str(r[k]) for k in fields) + " |\n")
    paired = [r for r in rows if r["equal"] != ""]
    equal = sum(1 for r in paired if r["equal"] is True)
    missing = len(rows) - len(paired)
    print(f"{unit}: {len(paired)} pairs, {equal} equal, {len(paired) - equal} differ, "
          f"{missing} without partner -> {out_dir}")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("units", nargs="*", help="unit ids, e.g. tool-brush")
    ap.add_argument("--all", action="store_true", help="every unit that has a reference folder")
    ap.add_argument("--threshold", type=int, default=16, help="per-pixel channel difference counted as different")
    ap.add_argument("--ref-root", type=Path, default=DEFAULT_REF)
    ap.add_argument("--fp-root", type=Path, default=DEFAULT_FP)
    ap.add_argument("--out-root", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--no-images", action="store_true", help="metrics only")
    args = ap.parse_args(argv)

    units = list(args.units)
    if args.all:
        roots = [args.ref_root, args.fp_root]
        units += sorted({p.name for r in roots if r.is_dir() for p in r.iterdir() if p.is_dir()})
    if not units:
        ap.error("give at least one unit or --all")
    rc = 0
    for unit in dict.fromkeys(units):
        rc |= compare_unit(unit, args.ref_root, args.fp_root, args.out_root,
                           args.threshold, not args.no_images)
    return rc


if __name__ == "__main__":
    sys.exit(main())
