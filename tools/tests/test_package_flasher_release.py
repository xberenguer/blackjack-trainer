"""Source-bound round release packaging rejects touch, stale source and extra writes."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('package_flasher', Path(__file__).resolve().parents[1] / 'package_flasher_release.py')
P = importlib.util.module_from_spec(spec)
spec.loader.exec_module(P)


class RoundPackage(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name).resolve()
        self.build = self.root / 'build'
        self.build.mkdir()
        self.inputs = {}
        for i in range(31):
            name = f'unit-{i}.rs'
            (self.root / name).write_text(f'// Source {i}\n')
            self.inputs[name] = P.sha((self.root / name).read_bytes())
        self.app = bytearray(529)
        self.app[0] = 0xe9
        self.app[3] = 5 << 4
        self.app[12] = 9
        self.app[32:36] = bytes.fromhex('3254cdab')
        self.app[48:54] = b'0.12.0'
        self.proof = dict(version='0.12.0', board='waveshare175c', application_offset=0x200000,
                          flash_bytes=0x2000000, display_orientation_degrees=270, touch_enabled=False,
                          flash_partition_table=False, source_inputs=self.inputs, artifacts={},
                          linked_stack_audit=dict(unknown_sp_mutations=0, app_decode_frame_bytes=64,
                                                 decoder_cap=32768, maximum_frame_bytes=128,
                                                 explicit_existing_path_cap=32768),
                          iram_dram_overlap_bytes=0, main_frame_bytes=1024, stack_bytes=262144)
        self.write()

    def tearDown(self):
        self.temp.cleanup()

    def write(self):
        name = 'stardust-waveshare175c.bin'
        (self.build / name).write_bytes(self.app)
        self.proof['artifacts'][name] = dict(bytes=len(self.app), sha256=P.sha(self.app))
        (self.build / 'manifest.json').write_text(json.dumps(self.proof))

    def package(self):
        return P.waveshare_release(self.root, self.build, '0.12.0')

    def test_app_only_contract_and_factory_map(self):
        row, data = self.package()
        self.assertEqual(row['contract'], 'blackjack-waveshare175c-v1')
        self.assertEqual(row['parts'], [dict(file='blackjack-waveshare175c-app.bin', sourceOffset=0,
                                            length=len(self.app), address=0x200000, kind='application')])
        self.assertEqual(set(data), {'blackjack-waveshare175c-app.bin'})
        self.assertEqual(len(row['partitions']), 7)
        self.assertFalse(any(p['label'].startswith('rust') for p in row['partitions']))

    def test_touch_orientation_or_partition_write_is_rejected(self):
        for key, value in [('touch_enabled', True), ('display_orientation_degrees', 0),
                           ('flash_partition_table', True), ('application_offset', 0x10000),
                           ('flash_bytes', 0x800000), ('board', 'sticks3')]:
            with self.subTest(key=key):
                before = copy.deepcopy(self.proof)
                self.proof[key] = value
                self.write()
                with self.assertRaises(AssertionError):
                    self.package()
                self.proof = before

    def test_changed_source_or_unqualified_stack_is_rejected(self):
        (self.root / 'unit-0.rs').write_text('// Changed\n')
        with self.assertRaisesRegex(AssertionError, 'Build source changed'):
            self.package()
        self.inputs['unit-0.rs'] = P.sha((self.root / 'unit-0.rs').read_bytes())
        self.proof['linked_stack_audit']['unknown_sp_mutations'] = 1
        self.write()
        with self.assertRaises(AssertionError):
            self.package()

    def test_wrong_image_capacity_or_embedded_version_is_rejected(self):
        self.app[3] = 3 << 4
        self.write()
        with self.assertRaises(AssertionError):
            self.package()
        self.app[3] = 5 << 4
        self.app[48:54] = b'0.11.3'
        self.write()
        with self.assertRaisesRegex(AssertionError, 'Embedded app version'):
            self.package()

    def test_application_cannot_extend_into_private_tail(self):
        self.app.extend(b'\xff' * (P.WAVESHARE_MAX_APP + 1 - len(self.app)))
        self.write()
        with self.assertRaisesRegex(AssertionError, 'release write boundary'):
            self.package()


if __name__ == '__main__':
    unittest.main()
