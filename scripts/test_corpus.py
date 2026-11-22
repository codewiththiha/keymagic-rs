"""Validate corpus provenance and the first upstream behavioral cases."""
import hashlib
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / 'corpus'
REVISION = '2db30dc8e8a5f091d1434a2bc1702b29cf113804'
EXPECTED_FILES = {
    'kms/upstream/simple_test.kms': '74f15abf97d603acedac95be01eb910d61363bf027a79486362a4e35680ca767',
    'kms/upstream/comprehensive_test.kms': 'f216c81dd097a65b60b81b937240e4c13a34481fcb2655748cc818258bd207a1',
    'kms/upstream/test_keyboard.kms': 'a29a60cc10445a4f67ebbfeb0bcd720d689153bc250519628c65ab27c3d91132',
    'km2/upstream/comprehensive_test.km2': '3b3f6c46ac7f56c7c6a1856438521a4525d6ac623a2f9d64c59ae470f8685b87',
}


def load(path):
    with path.open('rb') as file:
        return tomllib.load(file)


class CorpusTests(unittest.TestCase):
    def test_pinned_fixture_bytes_and_provenance(self):
        lines = (ROOT / 'sources.tsv').read_text().splitlines()
        self.assertEqual(lines[0].split('\t'), ['source', 'https://github.com/thantthet/keymagic-3', REVISION])
        entries = [line.split('\t') for line in lines[1:]]
        self.assertEqual({entry[2] for entry in entries}, set(EXPECTED_FILES))
        self.assertEqual(len(entries), len(EXPECTED_FILES))
        for kind, upstream, path, blob, digest in entries:
            with self.subTest(path=path):
                expected = EXPECTED_FILES[path]
                self.assertEqual(kind, 'file')
                self.assertEqual(digest, expected)
                self.assertTrue(upstream.startswith(('kms2km2/tests/', 'keymagic-core/tests/')))
                data = (ROOT / path).read_bytes()
                git_content = f'blob {len(data)}\0'.encode() + data
                self.assertEqual(hashlib.sha1(git_content).hexdigest(), blob)
                self.assertEqual(hashlib.sha256(data).hexdigest(), expected)

    def test_basic_cases_preserve_upstream_actions(self):
        data = load(ROOT / 'cases/basic_rules.toml')
        cases = {case['name']: case for case in data['case']}
        self.assertEqual(len(data['case']), 3)
        self.assertEqual(set(cases), {'simple_mapping', 'single_character', 'unmatched_character'})
        for case in cases.values():
            self.assertTrue((ROOT / case['keyboard']).is_file())
            self.assertTrue(case['upstream_test'].startswith('keymagic-core/tests/basic_rule_tests.rs::'))
        self.assertEqual(cases['simple_mapping']['events'], [
            {'key': 'k', 'composing': 'k', 'action': 'insert', 'text': 'k'},
            {'key': 'a', 'composing': 'က', 'action': 'replace', 'delete': 1, 'text': 'က'},
        ])
        self.assertEqual(cases['single_character']['events'], [
            {'key': 'a', 'composing': '\u200btest', 'action': 'insert', 'text': '\u200btest'},
        ])
        self.assertEqual(cases['unmatched_character']['events'], [
            {'key': 'x', 'composing': 'x', 'action': 'insert', 'text': 'x'},
        ])


if __name__ == '__main__':
    unittest.main()
