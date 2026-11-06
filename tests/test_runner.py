import copy
import json
from pathlib import Path
import tempfile
import unittest
from types import SimpleNamespace

import qa
from qa_report import compatibility, compare_reports, image_diff, render


class RunnerTests(unittest.TestCase):
    def test_real_rune_timing_format(self):
        log = 'rune: 360 frames at 60 fps, gpu init 18.2 ms, first frame 2.73 ms, frame cpu mean 0.130 ms p50 0.120 p95 0.290 max 2.730, 174 instances'
        metrics = qa.parse_timing(log)
        self.assertEqual(metrics['frame_p95_ms'], 0.29)
        self.assertEqual(metrics['instances'], 174)
        with self.assertRaises(ValueError):
            qa.parse_timing('GPU unavailable')

    def test_statistics_keep_samples_and_dispersion(self):
        result = qa.summarize([{'ms': x} for x in [2, 3, 4, 5, 100]])
        self.assertEqual(result['median']['ms'], 4)
        self.assertEqual(result['mad']['ms'], 1)
        self.assertEqual(len(result['samples']), 5)

    def test_gpu_identity_must_remain_stable(self):
        row = {}
        qa.record_gpu(row, 'rune: GPU Intel\nrune: a surface draws with Intel')
        self.assertEqual(len(row['gpu']), 2)
        with self.assertRaises(ValueError):
            qa.record_gpu(row, 'rune: GPU NVIDIA')
        with self.assertRaises(ValueError):
            qa.record_gpu({}, 'no adapter information')

    def test_assertions_reject_missing_and_wrong_states(self):
        tree = {'elements': [{'rect': [0, 0, 20, 20]}], 'accessibility': {
            'role': 'window', 'children': [{'role': 'switch', 'name': 'Sync', 'states': ['unchecked']}]}}
        assertion = {'match': {'role': 'switch', 'name': 'Sync'}, 'states': ['checked']}
        with self.assertRaises(ValueError):
            qa.assert_tree(tree, [assertion])
        tree['accessibility']['children'][0]['states'] = ['checked']
        qa.assert_tree(tree, [assertion])
        assertion['match']['name'] = 'missing'
        with self.assertRaises(ValueError):
            qa.assert_tree(tree, [assertion])

    def test_invalid_geometry_fails(self):
        for rect in ([0, 0, -2, 4], [0, float('nan'), 2, 4], [1, 2, 3]):
            with self.assertRaises(ValueError):
                qa.assert_tree({'elements': [{'rect': rect}]}, [])

    def test_unkeyed_target_uses_actual_bounds_and_requires_unique_match(self):
        event = {'t': 0.1, 'type': 'tap', 'target': {'label': 'Disabled'}}
        tree = {'elements': [{'label': 'Disabled', 'rect': [24, 80, 912, 16]}]}
        resolved = qa.resolve_targets([event], tree)
        self.assertEqual((resolved[0]['x'], resolved[0]['y']), (480, 88))
        self.assertIn('target', event)
        tree['elements'] *= 2
        with self.assertRaises(ValueError):
            qa.resolve_targets([event], tree)

    def test_existing_output_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as folder:
            with self.assertRaises(FileExistsError):
                qa.output_dir(folder, 'test')

    def test_failed_process_is_an_error_and_retains_log(self):
        with tempfile.TemporaryDirectory() as folder:
            log = Path(folder) / 'failure.log'
            with self.assertRaises(RuntimeError):
                qa.run([qa.sys.executable, '-c', 'print("evidence"); raise SystemExit(7)'], log=log)
            self.assertIn('evidence', log.read_text())

    def test_os_difference_is_allowed_but_workload_difference_is_not(self):
        report = {'kind': 'benchmark', 'metadata': {'os': 'Windows', 'settings': {'frames': 360},
                  'build': {'revision': 'abc', 'profile': 'perf', 'rustc': 'rustc 1.96.0\nhost: windows'}},
                  'results': [{'id': 'one'}]}
        other = copy.deepcopy(report)
        other['metadata']['os'] = 'Darwin'
        other['metadata']['build']['rustc'] = 'rustc 1.96.0\nhost: mac'
        self.assertEqual(compatibility(report, other), [])
        other['metadata']['settings']['frames'] = 180
        self.assertIn('different workload or rendering settings', compatibility(report, other))
        other['results'] = []
        self.assertIn('different case sets', compatibility(report, other))

    def test_render_escapes_untrusted_labels(self):
        report = {'kind': 'capture', 'metadata': {'label': '<script>alert(1)</script>', 'os': 'Windows',
                   'machine': 'AMD64', 'build': {'revision': 'abc', 'profile': 'perf'}}, 'results': []}
        html = render(report)
        self.assertNotIn('<script>alert(1)</script>', html)
        self.assertIn('&lt;script&gt;alert(1)', html)

    def test_native_comparison_flags_different_refresh_rates(self):
        report = {'kind': 'native', 'metadata': {'build': {'rustc': 'rustc 1.96'}},
                  'results': [{'id': 'loader', 'trace_header': {'hz': 240, 'scale': 1}}]}
        other = copy.deepcopy(report)
        other['results'][0]['trace_header']['hz'] = 120
        self.assertIn('different native hz: loader', compatibility(report, other))

    def test_case_ids_are_unique_and_events_finish_before_assertions(self):
        catalog = qa.read_json(qa.ROOT / 'cases.json')
        cases = catalog['smoke'] + catalog['interactions']
        self.assertEqual(len({c['id'] for c in cases}), len(cases))
        for case in catalog['interactions']:
            self.assertTrue(case['assert'])
            self.assertGreater(case['at'], max(e['t'] for e in case['events']))

    def test_pixel_diff_detects_rgb_and_alpha_changes(self):
        try:
            from PIL import Image
        except ImportError:
            self.skipTest('Install requirements.txt to exercise pixel comparisons')
        with tempfile.TemporaryDirectory() as folder:
            a, b, diff = [Path(folder) / name for name in ['a.png', 'b.png', 'diff.png']]
            im = Image.new('RGBA', (2, 2), (0, 0, 0, 255))
            im.save(a)
            im.save(b)
            self.assertEqual(image_diff(a, b, diff)['changed_percent'], 0)
            im.putpixel((0, 0), (1, 0, 0, 255))
            im.putpixel((1, 1), (0, 0, 0, 0))
            im.save(b)
            self.assertEqual(image_diff(a, b, diff)['changed_percent'], 50)
            Image.new('RGBA', (3, 2)).save(b)
            with self.assertRaises(ValueError):
                image_diff(a, b, diff)

    def test_native_trace_rejects_virtual_and_incomplete_runs(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'trace.jsonl'
            rows = [{'clock': 'wall', 'timestamp': 'cpu-submit'}] + [
                {'t': i * 16.7, 'quality': {'cpu_ms': i + 1}} for i in range(3)]
            path.write_text('\n'.join(json.dumps(row) for row in rows))
            values, _ = qa.trace_summary(path, 3)
            self.assertEqual(values['frame_mean_ms'], 2.5)
            with self.assertRaises(ValueError):
                qa.trace_summary(path, 4)
            rows[0]['clock'] = 'virtual'
            path.write_text('\n'.join(json.dumps(row) for row in rows))
            with self.assertRaises(ValueError):
                qa.trace_summary(path, 3)

    def test_comparison_reports_missing_cases_instead_of_silently_intersecting(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            report = {'kind': 'benchmark', 'metadata': {'label': 'before', 'settings': {},
                'build': {'revision': 'a', 'profile': 'perf', 'rustc': 'rustc 1.96'}},
                'results': [{'id': 'only-before', 'status': 'pass', 'median': {'frame_mean_ms': 1}, 'mad': {'frame_mean_ms': 0}}]}
            qa.write_json(root / 'before.json', report)
            report['results'] = []
            qa.write_json(root / 'after.json', report)
            out = root / 'comparison'
            out.mkdir()
            args = SimpleNamespace(before=root / 'before.json', after=root / 'after.json',
                                   allow_mismatch=True, max_changed_percent=None)
            with self.assertRaises(RuntimeError):
                compare_reports(args, out)
            saved = qa.read_json(out / 'comparison.json')
            self.assertEqual(saved['results'][0]['status'], 'fail')
            self.assertTrue((out / 'index.html').exists())


if __name__ == '__main__':
    unittest.main()
