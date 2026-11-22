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
ENGINE_TEST_FILES = {
    'auto_backspace_test.rs', 'backspace_history_test.rs', 'basic_rule_tests.rs',
    'common/engine_helpers.rs', 'common/mod.rs', 'ffi_tests.rs', 'km2_validation_tests.rs',
    'kms_loader_tests.rs', 'metadata_tests.rs', 'modifier_exact_match_test.rs',
    'state_tests.rs', 'variable_backref_test.rs', 'variable_tests.rs', 'virtual_key_tests.rs',
    'vowel_e_reordering_test.rs',
}


def load(path):
    with path.open('rb') as file:
        return tomllib.load(file)


class CorpusTests(unittest.TestCase):
    def test_pinned_fixture_bytes_and_provenance(self):
        lines = (ROOT / 'sources.tsv').read_text().splitlines()
        self.assertEqual(lines[0].split('\t'), ['source', 'https://github.com/thantthet/keymagic-3', REVISION])
        entries = [line.split('\t') for line in lines[1:]]
        expected_paths = set(EXPECTED_FILES) | {f'reference/km3/{name}' for name in ENGINE_TEST_FILES}
        self.assertEqual({entry[2] for entry in entries}, expected_paths)
        self.assertEqual(len(entries), len(expected_paths))
        for kind, upstream, path, blob, digest in entries:
            with self.subTest(path=path):
                self.assertEqual(kind, 'file')
                if path in EXPECTED_FILES:
                    self.assertEqual(digest, EXPECTED_FILES[path])
                else:
                    self.assertEqual(upstream, 'keymagic-core/tests/' + path.removeprefix('reference/km3/'))
                self.assertTrue(upstream.startswith(('kms2km2/tests/', 'keymagic-core/tests/')))
                data = (ROOT / path).read_bytes()
                git_content = f'blob {len(data)}\0'.encode() + data
                self.assertEqual(hashlib.sha1(git_content).hexdigest(), blob)
                self.assertEqual(hashlib.sha256(data).hexdigest(), digest)

    def test_upstream_bytes_are_not_normalized_by_git(self):
        attributes = (ROOT.parent / '.gitattributes').read_text().splitlines()
        for rule in ('corpus/reference/km3/** -text diff', 'corpus/kms/upstream/** -text diff',
                     'corpus/km2/upstream/** -text'):
            self.assertIn(rule, attributes)

    def test_upstream_engine_reference_contains_original_cases(self):
        source = (ROOT / 'reference/km3/basic_rule_tests.rs').read_text()
        for name in ('test_simple_string_mapping', 'test_single_char_mapping', 'test_no_matching_rule'):
            self.assertIn('fn ' + name + '()', source)
        self.assertEqual(sum((ROOT / 'reference/km3' / name).read_text().count('#[test]')
                             for name in ENGINE_TEST_FILES), 76)
        self.assertEqual({path.relative_to(ROOT / 'reference/km3').as_posix()
                          for path in (ROOT / 'reference/km3').rglob('*') if path.is_file()}, ENGINE_TEST_FILES)

    def test_basic_cases_preserve_upstream_actions(self):
        data = load(ROOT / 'cases/basic_rules.toml')
        cases = {case['name']: case for case in data['case']}
        self.assertEqual(len(data['case']), 18)
        self.assertEqual(len(cases), 18)
        self.assertTrue({'simple_mapping', 'single_character', 'unmatched_character',
                         'unicode_sequence', 'variable_substitution', 'priority_short',
                         'priority_long', 'greedy_match', 'recursive_chain', 'multi_replacement',
                         'variable_literal', 'variable_unicode', 'combined_consonant',
                         'combined_vowel', 'variable_output', 'zero_width_variable',
                         'mixed_ascii', 'mixed_unicode'} <= set(cases))
        for case in cases.values():
            self.assertTrue((ROOT / case['keyboard']).is_file())
            source, name = case['upstream_test'].split('::')
            self.assertTrue(source.startswith('keymagic-core/tests/'))
            reference = ROOT / 'reference/km3' / source.removeprefix('keymagic-core/tests/')
            self.assertIn('fn ' + name + '()', reference.read_text())
        self.assertEqual(cases['greedy_match']['events'][-1],
                         {'key': 'e', 'composing': 'Title', 'action': 'replace', 'delete': 4, 'text': 'Title'})
        self.assertEqual(cases['combined_vowel']['events'],
                         [{'key': 'ိ', 'action': 'insert', 'text': 'combined'}])
        self.assertEqual(cases['recursive_chain']['events'], [{'key': 'x', 'composing': 'X'}])
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
