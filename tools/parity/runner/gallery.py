"""index.html gallery over all units in the run folder (pairs side by side + difference + metrics)."""

from __future__ import annotations

import csv
import html
import json
from pathlib import Path

NOTABLE_SHARE = 1.0  # % of differing pixels from which a pair counts as notable


def _rows(udir: Path):
    log = {}
    lf = udir / "log.jsonl"
    if lf.exists():
        for line in lf.read_text(encoding="utf-8").splitlines():
            if line.strip():
                e = json.loads(line)
                log[e["step"]] = e
    metrics = {}
    sf = udir / "cmp" / "summary.csv"
    if sf.exists():
        with open(sf, encoding="utf-8") as f:
            for r in csv.DictReader(f):
                metrics[Path(r["name"]).stem] = r
    ids = list(dict.fromkeys(list(log) + list(metrics)))
    rows = []
    for i in ids:
        e, m = log.get(i, {}), metrics.get(i, {})
        res = e.get("result", {})
        bad = [f"{k}: {v.get('status')} – {v.get('error', '')}" for k, v in res.items() if v.get("status") not in ("ok", None)]
        failed = any(v.get("status") == "error" for v in res.values())
        try:
            share = float(m.get("diff_share_pct") or 0)
        except ValueError:
            share = 0.0
        notable = failed or share >= NOTABLE_SHARE or m.get("size_match") == "False" or bool(m.get("note"))
        rows.append({"id": i, "check": e.get("check") or "", "area": e.get("area") or "", "m": m, "share": share,
                     "bad": bad, "failed": failed, "notable": notable, "res": res})
    rows.sort(key=lambda r: (not r["failed"], not r["notable"], -r["share"]))
    return rows


def build(out_dir: Path) -> Path:
    units = []
    for udir in sorted(p for p in out_dir.iterdir() if p.is_dir() and (p / "log.jsonl").exists()):
        rows = _rows(udir)
        score = (sum(r["failed"] for r in rows), sum(r["notable"] for r in rows), max([r["share"] for r in rows] or [0]))
        units.append((score, udir.name, rows))
    units.sort(key=lambda u: (-u[0][0], -u[0][1], -u[0][2]))
    css = """
    :root{--bg:#f6f6f4;--fg:#1d1d1b;--mut:#6b6b66;--card:#fff;--line:#deded8;--bad:#c62828;--badbg:#fdecea;--warn:#a15c00;--warnbg:#fff4e0;--ok:#2e7d32}
    @media (prefers-color-scheme: dark){:root{--bg:#1b1b1a;--fg:#ececea;--mut:#a0a09a;--card:#252524;--line:#3a3a38;--badbg:#3a1d1b;--warnbg:#3a2e17}}
    body{margin:0;padding:16px;background:var(--bg);color:var(--fg);font:14px/1.45 system-ui,Segoe UI,sans-serif}
    h1{font-size:20px;margin:0 0 4px} h2{font-size:16px;margin:28px 0 8px} .mut{color:var(--mut)}
    nav a{margin-right:10px;white-space:nowrap} table{border-collapse:collapse;width:100%}
    .row{background:var(--card);border:1px solid var(--line);border-radius:6px;margin:8px 0;padding:8px 10px}
    .row.failed{border-color:var(--bad);background:var(--badbg)} .row.notable{border-left:4px solid var(--warn)}
    .head{display:flex;gap:12px;flex-wrap:wrap;align-items:baseline} .id{font-weight:600;font-family:Consolas,monospace}
    .err{color:var(--bad);font-family:Consolas,monospace;font-size:12px;white-space:pre-wrap}
    .imgs{overflow-x:auto;margin-top:6px} .imgs img{max-width:100%;height:auto;border:1px solid var(--line);image-rendering:pixelated}
    .pair{display:flex;gap:8px;flex-wrap:wrap} .pair figure{margin:0} figcaption{font-size:12px;color:var(--mut)}
    details{margin-top:4px} pre{font-size:12px;white-space:pre-wrap;max-height:240px;overflow:auto}
    .badge{font-size:12px;padding:1px 6px;border-radius:9px;background:var(--line)} .badge.bad{background:var(--bad);color:#fff}
    """
    out = ["<!doctype html><html lang='de'><head><meta charset='utf-8'><meta name='viewport' content='width=device-width,initial-scale=1'>",
           "<title>Parity Gallery</title><style>", css, "</style></head><body>",
           "<h1>Bildvergleich – Galerie</h1><p class='mut'>Je Einheit: Fehlschritte (rot) und auffällige Paare (≥ "
           f"{NOTABLE_SHARE:g} % abweichende Pixel, Größe anders) zuerst. Bild: Original | Firepixel | Differenz ×4.</p><nav>"]
    for (f, n, _), name, _rows_ in units:
        out.append(f"<a href='#{html.escape(name)}'>{html.escape(name)}</a><span class='mut'>({f}/{n})</span> ")
    out.append("</nav>")
    for (f, n, mx), name, rows in units:
        out.append(f"<h2 id='{html.escape(name)}'>{html.escape(name)} <span class='mut'>– {len(rows)} Schritte, "
                   f"{f} Fehler, {n} auffällig, max {mx:.2f} %</span></h2>")
        for r in rows:
            cls = "row" + (" failed" if r["failed"] else "") + (" notable" if r["notable"] and not r["failed"] else "")
            m = r["m"]
            metric = (f"<span class='badge{' bad' if r['notable'] else ''}'>{r['share']:.2f} %</span> "
                      f"<span class='mut'>mittel {m.get('mean_abs', '')} · max {m.get('max_abs', '')} · "
                      f"{m.get('ref_size', '')} / {m.get('fp_size', '')} {html.escape(m.get('note', ''))}</span>") if m else ""
            out.append(f"<div class='{cls}'><div class='head'><span class='id'>{html.escape(r['id'])}</span>"
                       f"<span>{html.escape(r['check'])}</span><span class='mut'>{html.escape(r['area'])}</span>{metric}</div>")
            for b in r["bad"]:
                out.append(f"<div class='err'>{html.escape(b)}</div>")
            cmp_img = Path(name) / "cmp" / f"{r['id']}.png"
            if (out_dir / cmp_img).exists():
                out.append(f"<div class='imgs'><img loading='lazy' src='{cmp_img.as_posix()}' alt='{html.escape(r['id'])}'></div>")
            else:
                figs = []
                for side, cap in (("orig", "Original"), ("fp", "Firepixel")):
                    p = Path(name) / side / f"{r['id']}.png"
                    if (out_dir / p).exists():
                        figs.append(f"<figure><img loading='lazy' src='{p.as_posix()}'><figcaption>{cap}</figcaption></figure>")
                if figs:
                    out.append("<div class='imgs pair'>" + "".join(figs) + "</div>")
            vals = {k: v.get("values") for k, v in r["res"].items() if v.get("values")}
            if vals:
                out.append("<details><summary>Werte</summary><pre>" + html.escape(json.dumps(vals, ensure_ascii=False, indent=1, default=str)[:6000]) + "</pre></details>")
            out.append("</div>")
    out.append("</body></html>")
    idx = out_dir / "index.html"
    idx.write_text("\n".join(out), encoding="utf-8")
    return idx
