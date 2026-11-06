#!/usr/bin/env python3
"""Portable QA for Rune's actual native component gallery. Python 3.11+."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import shutil
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parent
SOURCE = ROOT / '.upstream' / 'rune'
ARTIFACTS = ROOT / 'artifacts'
LOCK = json.loads((ROOT / 'rune.lock.json').read_text())
BUILD = ROOT / '.upstream' / 'build.json'
BINARY = SOURCE / 'target' / LOCK['profile'] / ('kit-gallery.exe' if os.name == 'nt' else 'kit-gallery')
NUMBER = r'(\d+(?:\.\d+)?)'
TIMING = re.compile(r'gpu init ' + NUMBER + r' ms, first frame ' + NUMBER
                    + r' ms, frame cpu mean ' + NUMBER + r' ms p50 ' + NUMBER
                    + r' p95 ' + NUMBER + r' max ' + NUMBER + r', (\d+) instances')
METRICS = ('gpu_init_ms', 'first_frame_ms', 'frame_mean_ms', 'frame_p50_ms', 'frame_p95_ms', 'frame_max_ms', 'instances')


def read_json(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def write_json(path, data):
    Path(path).write_text(json.dumps(data, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def run(command, *, cwd=ROOT, timeout=300, env=None, log=None, check=True):
    command = [str(x) for x in command]
    start = time.perf_counter()
    try:
        p = subprocess.run(command, cwd=cwd, env=env, capture_output=True,
                           text=True, encoding='utf-8', errors='replace', timeout=timeout)
    except subprocess.TimeoutExpired as error:
        output = error.stdout or b''
        if isinstance(output, bytes):
            output = output.decode('utf-8', errors='replace')
        if log:
            Path(log).write_text(output + f'\nTIMEOUT after {timeout}s\n', encoding='utf-8')
        raise RuntimeError(f'Timed out after {timeout}s: {command}') from error
    elapsed = (time.perf_counter() - start) * 1000
    output = p.stdout + p.stderr
    if log:
        Path(log).write_text(json.dumps(command) + '\n\n' + output, encoding='utf-8')
    if check and p.returncode:
        raise RuntimeError(f'Exit {p.returncode}: {command}\n{output[-6000:]}')
    return p, output, elapsed


def git(*args):
    return run(['git', '-C', SOURCE, *args])[0].stdout.strip()


def source_identity():
    if not (SOURCE / '.git').is_dir():
        raise RuntimeError('Rune is missing. Run: python qa.py setup')
    revision = git('rev-parse', 'HEAD')
    if revision != LOCK['revision']:
        raise RuntimeError('Rune checkout differs from rune.lock.json. Run setup after resolving local edits.')
    if git('status', '--porcelain', '--untracked-files=normal'):
        raise RuntimeError('Rune checkout has local changes. Keep upstream clean so results identify the exact source.')
    return revision


def setup(_):
    SOURCE.parent.mkdir(parents=True, exist_ok=True)
    if not (SOURCE / '.git').exists():
        print('Downloading the pinned Rune source...', flush=True)
        run(['git', 'clone', '--no-checkout', '--filter=blob:none', LOCK['repository'], SOURCE], timeout=600)
    elif git('status', '--porcelain'):
        raise RuntimeError('Refusing to change a checkout with local edits.')
    run(['git', '-C', SOURCE, 'fetch', '--depth', '1', 'origin', LOCK['revision']], timeout=600)
    run(['git', '-C', SOURCE, 'checkout', '--detach', LOCK['revision']], timeout=600)
    build(None)


def build(_):
    source_identity()
    ARTIFACTS.mkdir(exist_ok=True)
    print('Building Rune gallery (first build can take several minutes)...', flush=True)
    env = os.environ.copy()
    env['CARGO_TARGET_DIR'] = str(SOURCE / 'target')
    run(['cargo', 'build', '--manifest-path', SOURCE / 'Cargo.toml', '--locked',
         '--profile', LOCK['profile'], '-p', 'kit-gallery'], timeout=1800, env=env,
        log=ARTIFACTS / 'build.log')
    write_json(BUILD, {'revision': LOCK['revision'], 'profile': LOCK['profile'],
                       'binary_sha256': sha(BINARY), 'cargo_lock_sha256': sha(SOURCE / 'Cargo.lock'),
                       'rustc': run(['rustc', '-Vv'])[0].stdout.strip(),
                       'build_env': {k: os.environ.get(k) for k in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS')}})
    print(f'Ready: {BINARY}')


def require_binary():
    source_identity()
    if not BINARY.exists() or not BUILD.exists():
        raise RuntimeError('No verified build. Run: python qa.py build')
    info = read_json(BUILD)
    if info['revision'] != LOCK['revision'] or info['binary_sha256'] != sha(BINARY):
        raise RuntimeError('Build provenance changed. Run: python qa.py build')
    if info['cargo_lock_sha256'] != sha(SOURCE / 'Cargo.lock'):
        raise RuntimeError('Cargo lockfile differs from the recorded build. Run: python qa.py build')
    return info


def metadata(label):
    return {'schema': 1, 'created_utc': datetime.now(timezone.utc).isoformat(), 'label': label,
            'os': platform.system(), 'os_version': platform.version(), 'machine': platform.machine(),
            'processor': platform.processor(), 'logical_cpus': os.cpu_count(),
            'python': platform.python_version(), 'build': require_binary(),
            'qa_source_sha256': hashlib.sha256((ROOT / 'qa.py').read_bytes() + (ROOT / 'qa_report.py').read_bytes()).hexdigest(),
            'font_smoothing_default': 'on' if sys.platform == 'darwin' else 'off',
            'environment': {k: os.environ.get(k) for k in ('WGPU_BACKEND', 'RUNE_GPU_BACKEND', 'WGPU_ADAPTER_NAME', 'WGPU_POWER_PREF')}}


def output_dir(path, prefix):
    path = Path(path) if path else ARTIFACTS / f'{prefix}-{datetime.now().strftime("%Y%m%d-%H%M%S-%f")}'
    path = path.resolve()
    # Never silently mix a new run into a previous report.
    path.mkdir(parents=True, exist_ok=False)
    return path


def gallery_env():
    env = os.environ.copy()
    env['RUNE_GPU_LOG'] = '1'
    return env


def scenes():
    return run([BINARY, '--list-scenes'], env=gallery_env())[0].stdout.split()


def list_scenes(args):
    require_binary()
    for scene in scenes():
        if args.filter.lower() in scene.lower():
            print(scene)


def scene_args(scene, theme, scale, size, kit, reduced=False):
    args = ['--scene', scene, '--theme', theme, '--scale', str(scale), '--size', size, '--kit', kit]
    if reduced:
        args.append('--reduced-motion')
    return args


def gallery(args):
    require_binary()
    choices = scenes()
    scene = args.scene
    if not scene:
        choices = [s for s in choices if args.filter.lower() in s.lower()]
        if not choices:
            raise RuntimeError('No matching scenes.')
        for i, name in enumerate(choices, 1):
            print(f'{i:3}  {name}')
        answer = input('Open scene number: ').strip()
        if not answer.isdigit() or not 1 <= int(answer) <= len(choices):
            raise RuntimeError('Choose a number from the list.')
        scene = choices[int(answer) - 1]
    if scene not in choices:
        raise RuntimeError(f'Unknown scene: {scene}')
    command = [str(BINARY), *scene_args(scene, args.theme, args.scale, args.size, args.kit, args.reduced)]
    print('Close the native window to return. Native DPI follows the OS; --scale applies to headless runs.', flush=True)
    result = subprocess.run(command, cwd=ROOT, env=gallery_env())
    if result.returncode:
        raise RuntimeError(f'Gallery exited with {result.returncode}')


def flatten(node):
    if not isinstance(node, dict):
        return []
    return [node] + [entry for child in node.get('children', []) for entry in flatten(child)]


def assert_tree(tree, assertions):
    elements = tree.get('elements')
    if not isinstance(elements, list) or not elements:
        raise ValueError('Missing or empty element tree')
    for element in elements:
        rect = element.get('rect', [])
        if len(rect) != 4 or not all(isinstance(x, (int, float)) and math.isfinite(x) for x in rect) or min(rect[2:]) < 0:
            raise ValueError(f'Invalid rectangle: {element}')
    for assertion in assertions:
        nodes = flatten(tree.get('accessibility')) if assertion.get('tree', 'accessibility') == 'accessibility' else elements
        matches = [node for node in nodes if all(node.get(k) == v for k, v in assertion['match'].items())]
        if not matches:
            raise ValueError(f'No node matches {assertion["match"]}')
        for node in matches:
            if not all(state in node.get('states', []) for state in assertion.get('states', [])):
                raise ValueError(f'Expected states {assertion["states"]}: {node}')
            if any(node.get(k) != v for k, v in assertion.get('equals', {}).items()):
                raise ValueError(f'Expected {assertion["equals"]}: {node}')


def capture_case(folder, case, theme, scale, args):
    name = f'{case["id"]}-{theme}-{scale:g}x'
    command = [BINARY, *scene_args(case['scene'], theme, scale, args.size, args.kit, args.reduced),
               '--snap', folder / f'{name}.png', '--tree', folder / f'{name}.tree.json',
               '--at', str(case.get('at', 0.6))]
    result = {'id': name, 'scene': case['scene'], 'theme': theme, 'scale': scale,
              'status': 'pass', 'image': f'{name}.png', 'tree': f'{name}.tree.json', 'log': f'{name}.log'}
    try:
        if case.get('events'):
            events = case['events']
            if any('target' in event for event in events):
                initial = folder / f'{name}.initial.tree.json'
                run([BINARY, *scene_args(case['scene'], theme, scale, args.size, args.kit, args.reduced),
                     '--tree', initial, '--at', '0'], env=gallery_env(), log=folder / f'{name}.initial.log')
                events = resolve_targets(events, read_json(initial))
            script = folder / f'{name}.script.json'
            write_json(script, events)
            command += ['--script', script]
        _, output, elapsed = run(command, env=gallery_env(), log=folder / result['log'])
        tree = read_json(folder / result['tree'])
        assert_tree(tree, case.get('assert', []))
        with (folder / result['image']).open('rb') as stream:
            header = stream.read(24)
        expected = [round(float(n) * scale) for n in args.size.split('x')]
        dimensions = [int.from_bytes(header[16:20], 'big'), int.from_bytes(header[20:24], 'big')]
        if header[:8] != b'\x89PNG\r\n\x1a\n' or dimensions != expected:
            raise ValueError(f'PNG dimensions {dimensions}, expected {expected}')
        result.update(elapsed_ms=elapsed, image_sha256=sha(folder / result['image']), warnings=tree.get('warnings', []),
                      gpu=gpu_lines(output))
    except (RuntimeError, ValueError, OSError) as error:
        result.update(status='fail', error=str(error))
    print(f'{result["status"].upper():4} {name}', flush=True)
    return result


def resolve_targets(events, tree):
    """Resolve unkeyed fixture controls against the actual initial layout, not guessed coordinates."""
    resolved = []
    for original in events:
        event = dict(original)
        if 'target' in event:
            selector = event.pop('target')
            matches = [node for node in tree['elements'] if all(node.get(k) == v for k, v in selector.items())]
            if len(matches) != 1:
                raise ValueError(f'Expected exactly one initial target: {selector}, found {len(matches)}')
            x, y, w, h = matches[0]['rect']
            if min(w, h) <= 0:
                raise ValueError(f'Target has no hit area: {selector}')
            event.update(x=x + w / 2, y=y + h / 2)
        resolved.append(event)
    return resolved


def capture(args):
    meta = metadata(args.label)
    catalog = read_json(ROOT / 'cases.json')
    if args.all:
        cases = [{'id': scene, 'scene': scene} for scene in scenes()]
    else:
        cases = catalog['smoke'] + catalog['interactions']
    if args.only:
        cases = [case for case in cases if args.only in case['id'] or args.only in case['scene']]
    if not cases:
        raise RuntimeError('No matching cases.')
    meta['settings'] = {'size': args.size, 'kit': args.kit, 'reduced_motion': args.reduced,
                        'themes': args.themes, 'scales': args.scales, 'cases': cases}
    folder = output_dir(args.out, 'capture')
    report = {'kind': 'capture', 'metadata': meta, 'results': []}
    for case in cases:
        for theme in args.themes:
            for scale in args.scales:
                report['results'].append(capture_case(folder, case, theme, scale, args))
                save_report(folder, report)
    print(f'Report: {folder / "index.html"}')
    if any(r['status'] != 'pass' for r in report['results']):
        raise RuntimeError('Some cases failed. See the report and logs.')


def parse_timing(output):
    match = TIMING.search(output)
    if not match:
        raise ValueError('Rune timing summary missing; refusing to report a successful benchmark.')
    return dict(zip(METRICS, map(float, match.groups())))


def gpu_lines(output):
    return [line for line in output.splitlines() if line.startswith(('rune: GPU', 'rune: a surface draws with'))]


def record_gpu(row, output):
    gpu = gpu_lines(output)
    if not gpu:
        raise ValueError('GPU identity missing from Rune log')
    if row.get('gpu') and row['gpu'] != gpu:
        raise ValueError('GPU identity changed between samples; refusing to combine measurements')
    row['gpu'] = gpu


def summarize(samples):
    med = {key: statistics.median(sample[key] for sample in samples) for key in samples[0]}
    return {'median': med, 'mad': {key: statistics.median(abs(s[key] - med[key]) for s in samples) for key in med}, 'samples': samples}


def bench(args):
    meta = metadata(args.label)
    workloads = read_json(ROOT / 'cases.json')['benchmarks']
    if args.only:
        workloads = [w for w in workloads if args.only in w['id']]
    if not workloads:
        raise RuntimeError('No matching workloads.')
    last_event = max((event['t'] + event.get('dur', 0) for work in workloads for event in work.get('events', [])), default=0)
    if (args.frames - 1) / args.fps < last_event + 0.5:
        raise ValueError('Frame count would truncate the workload. Allow at least 0.5 seconds after the final event.')
    meta['settings'] = {'frames': args.frames, 'fps': args.fps, 'size': args.size, 'scale': args.scale,
                        'theme': args.theme, 'kit': args.kit, 'reduced_motion': args.reduced,
                        'warmups': 1, 'runs': args.runs, 'workloads': workloads}
    folder = output_dir(args.out, 'bench')
    report = {'kind': 'benchmark', 'metadata': meta, 'results': []}
    for work in workloads:
        row = {'id': work['id'], 'status': 'pass', 'samples': []}
        script = folder / f'{work["id"]}.script.json'
        write_json(script, work.get('events', []))
        try:
            for repeat in range(args.runs + 1):
                command = [BINARY, *scene_args(work['scene'], args.theme, args.scale, args.size, args.kit, args.reduced),
                           '--script', script, '--frames', str(args.frames), '--fps', str(args.fps)]
                _, output, elapsed = run(command, env=gallery_env(), log=folder / f'{work["id"]}-{repeat}.log')
                sample = parse_timing(output)
                sample['process_wall_ms'] = elapsed
                record_gpu(row, output)
                if repeat:
                    row['samples'].append(sample)
                print(f'{work["id"]}: {"warmup" if repeat == 0 else str(repeat) + "/" + str(args.runs)}', flush=True)
            row.update(summarize(row['samples']))
        except (RuntimeError, ValueError, OSError) as error:
            row.update(status='fail', error=str(error))
        report['results'].append(row)
        save_report(folder, report)
    print(f'Report: {folder / "index.html"}')
    if any(r['status'] != 'pass' for r in report['results']):
        raise RuntimeError('Benchmark failed. See the logs.')


def save_report(folder, report):
    from qa_report import render
    write_json(folder / 'results.json', report)
    (folder / 'index.html').write_text(render(report), encoding='utf-8')


def trace_summary(path, expected_frames):
    rows = [json.loads(line) for line in Path(path).read_text(encoding='utf-8').splitlines() if line.strip()]
    if not rows or rows[0].get('clock') != 'wall' or rows[0].get('timestamp') != 'cpu-submit':
        raise ValueError('Expected a native wall-clock CPU submission trace')
    frames = [r for r in rows if 'quality' in r]
    if len(frames) < expected_frames:
        raise ValueError(f'Incomplete trace: {len(frames)} / {expected_frames} frames')
    cpu = sorted(r['quality']['cpu_ms'] for r in frames[1:])
    intervals = sorted(b['t'] - a['t'] for a, b in zip(frames, frames[1:]))
    def p95(values):
        return values[math.ceil(len(values) * 0.95) - 1]
    values = {'frame_mean_ms': statistics.mean(cpu), 'frame_p50_ms': statistics.median(cpu),
              'frame_p95_ms': p95(cpu), 'frame_max_ms': max(cpu),
              'submit_interval_p50_ms': statistics.median(intervals), 'submit_interval_p95_ms': p95(intervals)}
    # Only record actual presentation metrics when Rune supplies them (currently macOS).
    presents = sorted(r['present_dt_ms'] for r in frames if isinstance(r.get('present_dt_ms'), (float, int)))
    if presents:
        values.update(present_interval_p50_ms=statistics.median(presents), present_interval_p95_ms=p95(presents))
    return values, rows[0]


def native(args):
    meta = metadata(args.label)
    if args.frames < 3:
        raise ValueError('Native tracing requires at least 3 frames')
    if args.scene not in scenes():
        raise ValueError(f'Unknown scene: {args.scene}')
    folder = output_dir(args.out, 'native')
    meta['settings'] = {'scene': args.scene, 'frames': args.frames, 'size': args.size, 'theme': args.theme,
                        'kit': args.kit, 'scale': 'OS', 'runs': args.runs, 'warmups': 1}
    report = {'kind': 'native', 'metadata': meta, 'results': []}
    row = {'id': args.scene, 'status': 'pass', 'samples': []}
    print('Recording native windows. Keep them visible and leave input idle. They close after the requested frames.', flush=True)
    try:
        for repeat in range(args.runs + 1):
            trace = folder / f'native-{repeat}.jsonl'
            command = [BINARY, *scene_args(args.scene, args.theme, 1, args.size, args.kit),
                       '--native', '--trace', trace, '--frames', str(args.frames)]
            _, log, _ = run(command, env=gallery_env(), log=folder / f'native-{repeat}.log', timeout=120)
            sample, header = trace_summary(trace, args.frames)
            if row.get('trace_header') and row['trace_header'] != header:
                raise ValueError('Native viewport or scale changed between runs')
            row['trace_header'] = header
            record_gpu(row, log)
            if repeat:
                row['samples'].append(sample)
            print(f'Native {repeat}/{args.runs} completed', flush=True)
        row.update(summarize(row['samples']))
    except (RuntimeError, ValueError, OSError) as error:
        row.update(status='fail', error=str(error))
    report['results'].append(row)
    save_report(folder, report)
    print(f'Report: {folder / "index.html"}')
    if row['status'] == 'fail':
        raise RuntimeError(row['error'])


def upstream_tests(args):
    source_identity()
    folder = output_dir(args.out, 'tests')
    targets = ['disabled', 'interactive', 'select_keyboard', 'dialog_select', 'popover_focus',
               'combobox', 'list_keyboard', 'text_input_press', 'segmented', 'switch_snap', 'tree', 'table']
    command = ['cargo', 'test', '--manifest-path', SOURCE / 'Cargo.toml', '--locked', '--profile', LOCK['profile'], '-p', 'rune-kit']
    for target in targets:
        command += ['--test', target]
    env = os.environ.copy()
    env['CARGO_TARGET_DIR'] = str(SOURCE / 'target')
    print('Running upstream component behavior tests...', flush=True)
    run(command, timeout=1800, env=env, log=folder / 'upstream-tests.log')
    print(f'Passed. Log: {folder / "upstream-tests.log"}')


def doctor(_):
    print(f'{platform.system()} {platform.release()} / {platform.machine()} / Python {platform.python_version()}')
    for command in ('git', 'rustc', 'cargo'):
        path = shutil.which(command)
        print(f'{command}: {path or "MISSING"}')
        if path:
            print(run([command, '--version'], check=False)[1].strip())
    print(f'Pinned Rune: {LOCK["revision"]}')
    print(f'Expected Rust for comparisons: {LOCK["rust_version"]}')
    try:
        import PIL
        print(f'Pixel diffs: Pillow {PIL.__version__}')
    except ImportError:
        print('Pixel diffs: install requirements.txt. Other commands use the Python standard library.')
    print(f'Gallery: {BINARY} ({"built" if BINARY.exists() else "not built"})')


def positive(value):
    value = int(value)
    if value < 1:
        raise argparse.ArgumentTypeError('must be positive')
    return value


def scale_value(value):
    value = float(value)
    if not math.isfinite(value) or not 0 < value <= 4:
        raise argparse.ArgumentTypeError('scale must be greater than 0 and at most 4')
    return value


def size_value(value):
    if not re.fullmatch(r'[1-9]\d{1,3}x[1-9]\d{1,3}', value) or any(int(n) > 4096 for n in value.split('x')):
        raise argparse.ArgumentTypeError('use WIDTHxHEIGHT, each between 10 and 4096')
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    for command, function in [('doctor', doctor), ('setup', setup), ('build', build)]:
        sub.add_parser(command).set_defaults(func=function)
    ls = sub.add_parser('list', help='list actual upstream scenes')
    ls.add_argument('filter', nargs='?', default='')
    ls.set_defaults(func=list_scenes)
    test = sub.add_parser('test', help='run selected upstream Rust behavior tests')
    test.add_argument('--out')
    test.set_defaults(func=upstream_tests)
    p = sub.add_parser('native', help='record real window CPU submission timing; requires a visible desktop')
    p.set_defaults(func=native)
    p.add_argument('scene', nargs='?', default='ui-loader--basic')
    p.add_argument('--frames', type=positive, default=240)
    p.add_argument('--runs', type=positive, default=3)
    p.add_argument('--size', type=size_value, default='960x720')
    p.add_argument('--theme', choices=['light', 'dark'], default='dark')
    p.add_argument('--kit', choices=['opencode', 'neutral', 'palette'], default='opencode')
    p.add_argument('--label', default=platform.system().lower())
    p.add_argument('--out')
    for command, function in [('gallery', gallery), ('capture', capture), ('bench', bench)]:
        p = sub.add_parser(command)
        p.set_defaults(func=function)
        p.add_argument('--size', type=size_value, default='960x720')
        p.add_argument('--kit', choices=['opencode', 'neutral', 'palette'], default='opencode')
        p.add_argument('--reduced', action='store_true')
        if command == 'capture':
            p.add_argument('--themes', nargs='+', choices=['light', 'dark'], default=['light', 'dark'])
            p.add_argument('--scales', type=scale_value, nargs='+', default=[1, 1.25, 2])
            p.add_argument('--all', action='store_true', help='capture every gallery scene; interactions are in the default suite')
        else:
            p.add_argument('--theme', choices=['light', 'dark'], default='dark')
            p.add_argument('--scale', type=scale_value, default=1)
        if command == 'gallery':
            p.add_argument('scene', nargs='?')
            p.add_argument('--filter', default='')
        else:
            p.add_argument('--out')
            p.add_argument('--label', default=platform.system().lower())
            p.add_argument('--only', default='')
        if command == 'bench':
            p.add_argument('--runs', type=positive, default=7)
            p.add_argument('--frames', type=positive, default=360)
            p.add_argument('--fps', type=positive, default=60)
    compare = sub.add_parser('compare', help='compare capture or benchmark results.json files')
    compare.add_argument('before', type=Path)
    compare.add_argument('after', type=Path)
    compare.add_argument('--out')
    compare.add_argument('--allow-mismatch', action='store_true', help='compare different revisions/settings with a visible warning')
    compare.add_argument('--max-changed-percent', type=float, default=None, help='optional visual CI failure threshold')
    def compare_command(args):
        from qa_report import compare_reports
        compare_reports(args, output_dir(args.out, 'compare'))
    compare.set_defaults(func=compare_command)
    args = parser.parse_args()
    try:
        args.func(args)
    except (RuntimeError, ValueError, OSError) as error:
        print(f'ERROR: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
