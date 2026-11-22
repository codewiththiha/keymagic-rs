"""Validate corpus provenance and the first upstream behavioral cases."""
import hashlib
import json
import re
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

    def test_engine_test_references_are_traced(self):
        files = ('auto_backspace_test.rs', 'backspace_history_test.rs', 'basic_rule_tests.rs',
                 'modifier_exact_match_test.rs', 'state_tests.rs', 'variable_backref_test.rs',
                 'variable_tests.rs', 'virtual_key_tests.rs', 'vowel_e_reordering_test.rs')
        covered = set()
        for path in (ROOT / 'cases').glob('*.toml'):
            for case in load(path)['case']:
                covered.update(case.get('also_tests', []))
                if 'upstream_test' in case:
                    covered.add(case['upstream_test'])
        skipped = {'keymagic-core/tests/virtual_key_tests.rs::test_complex_modifier_combination'}
        for filename in files:
            source = (ROOT / 'reference/km3' / filename).read_text()
            for name in re.findall(r'#\[test\]\s*fn\s+(\w+)', source):
                reference = 'keymagic-core/tests/' + filename + '::' + name
                with self.subTest(test=reference):
                    self.assertTrue(reference in covered or reference in skipped)
        source = (ROOT / 'reference/km3/virtual_key_tests.rs').read_text()
        body = source.split('fn test_complex_modifier_combination() {', 1)[1].split('}', 1)[0]
        self.assertFalse(re.sub(r'//[^\n]*', '', body).strip())

    def test_all_scenarios_have_unique_inputs_and_assertions(self):
        names, signatures = set(), set()
        inputs = {'key', 'vk', 'modifiers', 'reset', 'set_composing', 'preview'}
        outputs = {'action', 'delete', 'text', 'composing', 'processed', 'engine_composing'}
        for path in sorted((ROOT / 'cases').glob('*.toml')):
            for case in load(path)['case']:
                with self.subTest(case=case['name']):
                    self.assertNotIn(case['name'], names)
                    names.add(case['name'])
                    self.assertTrue(case.get('upstream_test') or case.get('upstream_source'))
                    self.assertTrue(case['events'])
                    self.assertTrue(any(outputs & event.keys() for event in case['events']))
                    sequence = [{k: v for k, v in event.items() if k in inputs}
                                for event in case['events']]
                    keyboard = (ROOT / case['keyboard']).read_bytes()
                    signature = (hashlib.sha256(keyboard).digest(), json.dumps(sequence, sort_keys=True))
                    self.assertNotIn(signature, signatures)
                    signatures.add(signature)
        self.assertGreaterEqual(len(signatures), 150)

    def test_vowel_reordering_cases_preserve_upstream_assertions(self):
        cases = {case['name']: case for case in load(ROOT / 'cases/vowel_reordering.toml')['case']}
        self.assertEqual(set(cases), {'vowel_basic', 'vowel_consonant', 'vowel_stacked',
                                     'vowel_medial', 'vowel_complex', 'vowel_two_consonants',
                                     'vowel_remove_filler'})
        self.assertEqual(cases['vowel_stacked']['events'][-1],
                         {'key': 'u', 'composing': 'က္ကေ', 'action': 'replace',
                          'delete': 2, 'text': '္ကေ'})
        self.assertEqual(cases['vowel_remove_filler']['events'][-1],
                         {'key': 'O', 'action': 'replace', 'delete': 2, 'text': 'ဿေ'})
        for case in cases.values():
            self.assertTrue((ROOT / case['keyboard']).is_file())
            self.assertIn('fn ' + case['upstream_test'].split('::')[1] + '()',
                          (ROOT / 'reference/km3/vowel_e_reordering_test.rs').read_text())

    def test_any_exhausts_printable_ascii_and_boundaries(self):
        cases = load(ROOT / 'cases/any_ascii.toml')['case']
        by_key = {case['events'][1]['key']: case for case in cases}
        expected = {chr(code) for code in range(0x21, 0x7f)} | {' ', '\x1f', '\x7f', 'က'}
        self.assertEqual(len(cases), 98)
        self.assertEqual(set(by_key), expected)
        self.assertEqual(len({case['name'] for case in cases}), len(cases))
        for key, case in by_key.items():
            with self.subTest(codepoint=ord(key)):
                self.assertEqual(case['keyboard'], 'kms/cases/any_ascii.kms')
                self.assertIn('keymagic-core/src/engine/matching/matcher.rs', case['upstream_source'])
                self.assertEqual(case['events'][0],
                                 {'key': '~', 'composing': '~', 'action': 'insert', 'text': '~'})
                event = case['events'][1]
                if '!' <= key <= '~':
                    self.assertEqual(event, {'key': key, 'composing': 'က' + key,
                                             'action': 'replace', 'delete': 1, 'text': 'က' + key})
                else:
                    self.assertEqual(event, {'key': key, 'composing': '~' + key,
                                             'action': 'insert', 'text': key})

    def test_history_cases_preserve_upstream_assertions(self):
        cases = {case['name']: case for case in load(ROOT / 'cases/history.toml')['case']}
        self.assertEqual(set(cases), {'history_restore', 'history_disabled', 'history_reset',
                                     'history_set', 'history_bounded', 'history_preview',
                                     'history_rule', 'history_not_recorded'})
        self.assertEqual(cases['history_preview']['events'][-1],
                         {'vk': 2, 'preview': True, 'composing': 'a', 'engine_composing': 'ab'})
        self.assertEqual(cases['history_bounded']['events'][-1]['composing'], 'abcd')
        self.assertEqual(cases['history_reset']['events'][2], {'reset': True, 'composing': ''})
        for case in cases.values():
            self.assertTrue((ROOT / case['keyboard']).is_file())
            self.assertIn('fn ' + case['upstream_test'].split('::')[1] + '()',
                          (ROOT / 'reference/km3/backspace_history_test.rs').read_text())

    def test_modifier_cases_preserve_upstream_assertions(self):
        cases = {case['name']: case for case in load(ROOT / 'cases/modifiers.toml')['case']}
        self.assertEqual(set(cases), {'shift_a', 'ctrl_a', 'shift_ctrl_a',
                                     'shift_alt_no_match', 'plain_a', 'all_no_match',
                                     'caps_lock_shift_a', 'vk_before_character',
                                     'vk_f1', 'vk_sequence', 'vk_shift_rule', 'vk_test_text',
                                     'vk_modifier_only_ignored'})
        self.assertEqual(cases['shift_alt_no_match']['events'], [
            {'vk': 26, 'modifiers': ['shift', 'alt'], 'composing': '', 'processed': False},
        ])
        self.assertEqual(cases['caps_lock_shift_a']['events'], [
            {'vk': 26, 'key': 'a', 'modifiers': ['shift', 'caps_lock'], 'composing': 'Shift+A'},
        ])
        for case in cases.values():
            self.assertTrue((ROOT / case['keyboard']).is_file())
            source, name = case['upstream_test'].split('::')
            path = ROOT / 'reference/km3' / source.removeprefix('keymagic-core/tests/')
            self.assertIn('fn ' + name + '()', path.read_text())

    def test_backspace_cases_preserve_upstream_sequences(self):
        cases = {case['name']: case for case in load(ROOT / 'cases/backspace.toml')['case']}
        self.assertEqual(set(cases), {'smart_undo', 'plain_delete', 'empty_backspace',
                                     'successive_deletes', 'normal_key_with_smart',
                                     'smart_without_history'})
        self.assertIn('keymagic-core/tests/auto_backspace_test.rs::test_auto_backspace_with_successful_match',
                      cases['smart_undo']['also_tests'])
        self.assertEqual(cases['smart_undo']['events'][-2:], [
            {'vk': 2, 'composing': 'k', 'action': 'replace', 'delete': 1, 'text': 'k', 'processed': True},
            {'vk': 2, 'composing': '', 'action': 'delete', 'delete': 1, 'processed': True},
        ])
        self.assertEqual(cases['empty_backspace']['events'],
                         [{'vk': 2, 'composing': '', 'action': 'none', 'processed': False}])
        for case in cases.values():
            self.assertTrue((ROOT / case['keyboard']).is_file())
            self.assertIn('fn ' + case['upstream_test'].split('::')[1] + '()',
                          (ROOT / 'reference/km3/auto_backspace_test.rs').read_text())

    def test_state_cases_preserve_upstream_sequences(self):
        cases = {case['name']: case for case in load(ROOT / 'cases/states.toml')['case']}
        self.assertEqual(set(cases), {'state_lifetime', 'multiple_states', 'state_any',
                                     'state_digits', 'active_states', 'state_priority'})
        self.assertEqual(cases['state_lifetime']['events'], [
            {'key': '1', 'action': 'insert', 'text': '1', 'composing': '1'},
            {'vk': 92, 'action': 'none', 'composing': '1'},
            {'key': '1', 'action': 'insert', 'text': '၁', 'composing': '1၁'},
            {'key': '1', 'action': 'insert', 'text': '1', 'composing': '1၁1'},
        ])
        self.assertEqual(cases['state_any']['events'][-1],
                         {'key': 'x', 'action': 'insert', 'text': 'xx', 'composing': 'xx'})
        self.assertEqual(cases['active_states']['events'][-1],
                         {'key': 'x', 'action': 'insert', 'text': 'BOTH', 'composing': 'xBOTH'})
        self.assertEqual(cases['state_priority']['events'][-1],
                         {'key': 't', 'action': 'insert', 'text': 'state'})
        self.assertEqual(cases['state_digits']['events'][-1],
                         {'key': '2', 'action': 'insert', 'text': 'ဎ္ဎ', 'composing': '၁၂ဍ္ဍ၂ဎ္ဎ'})
        for case in cases.values():
            self.assertTrue((ROOT / case['keyboard']).is_file())
            self.assertIn('fn ' + case['upstream_test'].split('::')[1] + '()',
                          (ROOT / 'reference/km3/state_tests.rs').read_text())

    def test_backreference_cases_preserve_upstream_sequences(self):
        cases = {case['name']: case for case in load(ROOT / 'cases/backrefs.toml')['case']}
        self.assertEqual(set(cases), {'positional_backrefs', 'multiple_backrefs'})
        self.assertEqual(cases['positional_backrefs']['events'], [
            {'key': 'q', 'action': 'insert', 'text': 'ဆ'},
            {'key': 'w', 'action': 'insert', 'text': 'တ'},
            {'key': 'e', 'action': 'insert', 'text': 'န'},
            {'key': 'y', 'action': 'insert', 'text': 'ပ'},
        ])
        self.assertEqual(cases['multiple_backrefs']['events'], [
            {'key': 'k', 'action': 'insert', 'text': 'k'},
            {'key': 'a', 'action': 'replace', 'delete': 1, 'text': 'ကာ'},
            {'key': 'g', 'action': 'insert', 'text': 'g'},
            {'key': 'i', 'action': 'replace', 'delete': 1, 'text': 'ဂိ'},
        ])
        for case in cases.values():
            self.assertTrue((ROOT / case['keyboard']).is_file())
            self.assertIn('fn ' + case['upstream_test'].split('::')[1] + '()',
                          (ROOT / 'reference/km3/variable_backref_test.rs').read_text())

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
