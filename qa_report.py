"""Static, offline reports. Pixel comparisons require Pillow; rendering does not."""
from html import escape
import json
import math
from pathlib import Path
import shutil

STYLE = '''
:root{color-scheme:dark;font:15px/1.55 system-ui,sans-serif;background:#101318;color:#e0e6ed}
body{max-width:1440px;margin:auto;padding:36px}h1{font-size:32px;letter-spacing:-1px}h2{font-size:18px}
a{color:#93c5fd}code,pre{font:12px/1.6 ui-monospace,monospace}pre{white-space:pre-wrap;overflow-wrap:anywhere}
.muted{color:#a3afbd}.pass{color:#86efac}.fail{color:#fda4af}.warn{color:#fcd34d}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(360px,1fr));gap:16px}
article,section{background:#191f28;border:1px solid #303a48;border-radius:12px;padding:20px;margin:16px 0}
article{margin:0;min-width:0}img{width:100%;height:auto;background:#272f3a;border-radius:6px}
table{border-collapse:collapse;width:100%;font-variant-numeric:tabular-nums}td,th{text-align:left;padding:10px;border-bottom:1px solid #303a48}
.pair{display:grid;grid-template-columns:1fr 1fr;gap:12px}.bar{height:12px;min-width:1px;background:#7db5ff;border-radius:3px;margin:4px 0}.after{background:#b6a2ff}
input{padding:12px;width:min(500px,90%);background:#101318;color:inherit;border:1px solid #536277;border-radius:8px}
details{margin-top:12px}summary{cursor:pointer}nav{display:flex;gap:20px}.scroll{overflow:auto}
'''


def e(value):
    return escape(str(value), quote=True)


def shell(title, body):
    return f'''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{e(title)}</title><style>{STYLE}</style><body><p class="muted">RUNE / NATIVE COMPONENT QA</p><h1>{e(title)}</h1>{body}
<script>document.querySelector('#filter')?.addEventListener('input', ev => {{
document.querySelectorAll('[data-case]').forEach(el => el.hidden = !el.dataset.case.includes(ev.target.value.toLowerCase()));
}});</script></body></html>'''


def render(report):
    meta = report['metadata']
    results = report['results']
    failures = sum(r['status'] != 'pass' for r in results)
    body = f'<p>{e(meta["label"])} · {e(meta["os"])} {e(meta["machine"])} · {len(results)} cases · <span class="{"fail" if failures else "pass"}">{failures} failures</span></p>'
    body += f'<p class="muted">Rune <code>{e(meta["build"]["revision"])}</code> · profile {e(meta["build"]["profile"])}</p>'
    body += '<p><a href="results.json">Raw results and environment</a></p><input id="filter" aria-label="Filter results" placeholder="Filter component or workload…">'
    if report['kind'] == 'capture':
        body += '<p class="muted">Pass means rendering and the specified tree assertions succeeded. Review images for visual defects. No visual baseline has been approved automatically.</p><div class="grid">'
        for row in results:
            body += f'<article data-case="{e(row["id"].lower())}"><h2>{e(row["id"])}</h2><p class="{row["status"]}">{e(row["status"].upper())}</p>'
            if row.get('image_sha256'):
                body += f'<a href="{e(row["image"])}"><img loading="lazy" alt="{e(row["id"])}" src="{e(row["image"])}"></a>'
            body += f'<p><a href="{e(row["tree"])}">Element and accessibility tree</a> · <a href="{e(row["log"])}">Log</a></p>'
            if row.get('warnings'):
                body += f'<pre class="warn">{e(json.dumps(row["warnings"], indent=2))}</pre>'
            if row.get('error'):
                body += f'<pre class="fail">{e(row["error"])}</pre>'
            body += '</article>'
        body += '</div>'
    else:
        if report['kind'] == 'native':
            body += '<p class="muted">Native wall-clock CPU submission trace. Submission intervals include idle time and are not displayed FPS. Windows has no presentation timestamps here. macOS presentation intervals appear only when supplied by Rune. One process warmup and each run’s first frame are excluded. Keep the window visible and input idle.</p>'
        else:
            body += '<p class="muted">Headless CPU update/build/encode/submit time. Excludes GPU completion, scanout, and input-to-display latency. One process warmup is discarded per workload. Rune’s mean excludes the first frame; its p50, p95, and max include it. Virtual FPS is a simulation step, not measured FPS.</p>'
        for row in results:
            body += f'<section data-case="{e(row["id"])}"><h2>{e(row["id"])}</h2>'
            if row['status'] == 'pass':
                body += '<table><tr><th>Metric</th><th>Median</th><th>MAD across runs</th></tr>'
                for key, value in row['median'].items():
                    body += f'<tr><td>{e(key)}</td><td>{value:.3f}</td><td>{row["mad"][key]:.3f}</td></tr>'
                body += '</table>'
                body += f'<p class="muted">{e(" / ".join(row.get("gpu", [])))}</p>'
                if row.get('trace_header'):
                    body += f'<p class="muted">Native trace: {e(json.dumps(row["trace_header"]))}</p>'
            else:
                body += f'<pre class="fail">{e(row.get("error", "Failed"))}</pre>'
            body += '</section>'
    body += f'<details><summary>Run metadata</summary><pre>{e(json.dumps(meta, indent=2))}</pre></details>'
    return shell('Component captures' if report['kind'] == 'capture' else 'Performance measurements', body)


def compatibility(before, after):
    reasons = []
    if before['kind'] != after['kind']:
        return ['different report kinds']
    a, b = before['metadata'], after['metadata']
    for field in ('revision', 'profile', 'cargo_lock_sha256', 'build_env'):
        if a['build'].get(field) != b['build'].get(field):
            reasons.append(f'different build {field}')
    # Rust's host triple differs across OSes; the version line must still agree.
    if a['build']['rustc'].splitlines()[0] != b['build']['rustc'].splitlines()[0]:
        reasons.append('different Rust compiler version')
    if a.get('qa_source_sha256') != b.get('qa_source_sha256'):
        reasons.append('different QA runner code')
    if a.get('settings') != b.get('settings'):
        reasons.append('different workload or rendering settings')
    if a.get('environment') != b.get('environment'):
        reasons.append('different renderer environment overrides')
    if {r['id'] for r in before['results']} != {r['id'] for r in after['results']}:
        reasons.append('different case sets')
    if before['kind'] == 'native':
        left = {r['id']: r for r in before['results']}
        right = {r['id']: r for r in after['results']}
        for key in left.keys() & right.keys():
            for field in ('width', 'height', 'scale', 'hz'):
                if left[key].get('trace_header', {}).get(field) != right[key].get('trace_header', {}).get(field):
                    reasons.append(f'different native {field}: {key}')
    return reasons


def image_diff(before, after, out):
    try:
        from PIL import Image, ImageChops, ImageStat
    except ImportError as error:
        raise RuntimeError('Pixel comparison needs: python -m pip install -r requirements.txt') from error
    with Image.open(before) as a, Image.open(after) as b:
        if a.size != b.size:
            raise ValueError(f'Image dimensions differ: {a.size} vs {b.size}')
        a, b = a.convert('RGBA'), b.convert('RGBA')
        difference = ImageChops.difference(a, b)
        bands = difference.split()
        maximum = bands[0]
        for band in bands[1:]:
            maximum = ImageChops.lighter(maximum, band)
        histogram = maximum.histogram()
        changed = sum(histogram[1:])
        amplified = maximum.point(lambda v: min(255, v * 8))
        amplified.save(out)
        return {'changed_percent': 100 * changed / (a.width * a.height),
                'mean_channel_error': sum(ImageStat.Stat(difference).mean) / 4,
                'max_channel_error': max(hi for lo, hi in difference.getextrema())}


def report_path(path):
    return path / 'results.json' if path.is_dir() else path


def compare_reports(args, folder):
    from qa import read_json, write_json
    a_path, b_path = report_path(args.before.resolve()), report_path(args.after.resolve())
    a, b = read_json(a_path), read_json(b_path)
    if a['kind'] != b['kind']:
        raise RuntimeError('Report kinds differ. Native, headless benchmark, and capture results cannot be mixed.')
    reasons = compatibility(a, b)
    if reasons and not args.allow_mismatch:
        raise RuntimeError('Incompatible comparison: ' + ', '.join(reasons) + '. Use --allow-mismatch for an explicitly exploratory comparison.')
    threshold = args.max_changed_percent
    if threshold is not None and (not math.isfinite(threshold) or not 0 <= threshold <= 100 or a['kind'] != 'capture'):
        raise ValueError('--max-changed-percent must be 0..100 and applies only to captures')
    body = f'<p>Before: {e(a["metadata"]["label"])} · After: {e(b["metadata"]["label"])}</p>'
    body += '<p class="muted">Different machines measure hardware, driver, OS, and platform defaults together. This comparison cannot isolate the OS cost. Font smoothing differs by platform. Pixel differences need human review. A benchmark PASS means samples were comparable, not that performance improved.</p>'
    if reasons:
        body += f'<p class="warn">Exploratory comparison: {e(", ".join(reasons))}</p>'
    left, right = ({r['id']: r for r in report['results']} for report in (a, b))
    results = []
    for index, key in enumerate(sorted(left.keys() | right.keys())):
        row = {'id': key, 'status': 'pass'}
        body += f'<section><h2>{e(key)}</h2>'
        if key not in left or key not in right or left[key]['status'] != 'pass' or right[key]['status'] != 'pass':
            row.update(status='fail', error='Case missing or failed in one of the input runs')
            body += f'<p class="fail">{e(row["error"])}</p>'
        elif a['kind'] == 'capture':
            before, after, diff = (f'{index}-{suffix}.png' for suffix in ('before', 'after', 'diff'))
            shutil.copyfile(a_path.parent / left[key]['image'], folder / before)
            shutil.copyfile(b_path.parent / right[key]['image'], folder / after)
            try:
                row.update(image_diff(folder / before, folder / after, folder / diff))
                if threshold is not None and row['changed_percent'] > threshold:
                    row['status'] = 'fail'
                body += f'<p class="{row["status"]}">{row["changed_percent"]:.3f}% pixels differ · mean channel error {row["mean_channel_error"]:.3f}/255</p>'
                body += f'<div class="pair"><div>Before<img alt="Before {e(key)}" src="{before}"></div><div>After<img alt="After {e(key)}" src="{after}"></div></div><details><summary>Difference, amplified 8×</summary><img alt="Pixel difference" src="{diff}"></details>'
            except ValueError as error:
                row.update(status='fail', error=str(error))
                body += f'<p class="fail">{e(error)}</p>'
        else:
            row['metrics'] = {}
            body += f'<p class="muted">Before GPU: {e(" / ".join(left[key].get("gpu", [])))}<br>After GPU: {e(" / ".join(right[key].get("gpu", [])))}</p>'
            body += '<p class="muted">Blue = before, purple = after. Bars share a scale within each metric. Δ = (after / before − 1) × 100.</p><table><tr><th>Metric</th><th>Before ± MAD</th><th>After ± MAD</th><th>Δ</th><th>Relative value</th></tr>'
            for metric in sorted(left[key]['median'].keys() & right[key]['median'].keys()):
                old = left[key]['median'][metric]
                new = right[key]['median'][metric]
                delta = (new / old - 1) * 100 if old else None
                row['metrics'][metric] = {'before': old, 'after': new, 'delta_percent': delta}
                bound = max(old, new, 0.001)
                change = f'{delta:+.1f}%' if delta is not None else 'n/a'
                body += f'<tr><td>{e(metric)}</td><td>{old:.3f} ± {left[key]["mad"][metric]:.3f}</td><td>{new:.3f} ± {right[key]["mad"][metric]:.3f}</td><td>{change}</td><td style="width:25%"><div class="bar" style="width:{old / bound * 100:.1f}%"></div><div class="bar after" style="width:{new / bound * 100:.1f}%"></div></td></tr>'
            body += '</table>'
            exclusive = left[key]['median'].keys() ^ right[key]['median'].keys()
            if exclusive:
                body += f'<p class="warn">Not available on both sides: {e(", ".join(sorted(exclusive)))}</p>'
        body += '</section>'
        results.append(row)
    write_json(folder / 'comparison.json', {'kind': a['kind'], 'warnings': reasons,
               'before': a['metadata'], 'after': b['metadata'], 'results': results})
    (folder / 'index.html').write_text(shell('Run comparison', body), encoding='utf-8')
    print(f'Report: {folder / "index.html"}')
    if any(row['status'] == 'fail' for row in results):
        raise RuntimeError('Comparison contains failed, missing, or over-threshold cases.')
