"""Tests for comparing transcribed cases with a reference engine."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from reference_cases import compare_events, decode_output, encode_events


class ReferenceCaseTests(unittest.TestCase):
    def test_encode_character_and_virtual_key_events(self):
        self.assertEqual(encode_events([{'key': 'က'}, {'vk': 92}, {'key': '1', 'vk': 17}]),
                         ['c:က', 'v:92', 'k:17:1'])
        for event in ({'key': 'ab'}, {'vk': -1}, {'vk': 65536}, {'key': 'a', 'vk': -1}, {}):
            with self.subTest(event=event), self.assertRaises(ValueError):
                encode_events([event])

    def test_encode_modifiers_with_and_without_characters(self):
        self.assertEqual(encode_events([
            {'vk': 26, 'key': 'A', 'modifiers': ['shift']},
            {'vk': 26, 'modifiers': ['ctrl', 'alt']},
            {'vk': 26, 'key': 'a', 'modifiers': ['shift', 'caps_lock']},
        ]), ['m:1:26:A', 'm:6:26:', 'm:9:26:a'])
        for event in ({'key': 'a', 'modifiers': ['shift']},
                      {'vk': 26, 'modifiers': ['unknown']},
                      {'vk': 26, 'modifiers': ['shift', 'shift']},
                      {'vk': 26, 'modifiers': 'shift'}):
            with self.subTest(event=event), self.assertRaises(ValueError):
                encode_events([event])

    def test_encode_reset_set_and_preview(self):
        self.assertEqual(encode_events([{'reset': True}, {'set_composing': 'xyz'},
                                        {'vk': 2, 'preview': True}]),
                         ['r:', 's:xyz', 't:v:2'])
        for event in ({'reset': False}, {'reset': True, 'key': 'a'},
                      {'set_composing': 3}, {'preview': True},
                      {'vk': 2, 'preview': 'true'}):
            with self.subTest(event=event), self.assertRaises(ValueError):
                encode_events([event])

    def test_decodes_unicode_actions_and_ignores_reference_logs(self):
        output = 'upstream log\nCORPUS\tinsert\t0\te18080\te18080\ttrue\te18080\n'
        self.assertEqual(decode_output(output), [
            {'action': 'insert', 'delete': 0, 'text': 'က', 'composing': 'က',
             'processed': True, 'engine_composing': 'က'},
        ])

    def test_checks_only_fields_asserted_by_upstream(self):
        actual = [{'action': 'insert', 'delete': 0, 'text': 'k', 'composing': 'k'},
                  {'action': 'replace', 'delete': 1, 'text': 'က', 'composing': 'က'}]
        self.assertEqual(compare_events([{'key': 'k'}, {'key': 'a', 'delete': 1}], actual), [])
        self.assertTrue(compare_events([{'key': 'k', 'composing': 'bad'}], actual))
        self.assertTrue(compare_events([{'key': 'k'}, {'key': 'a', 'delete': 2}], actual))
        self.assertTrue(compare_events([{'processed': True}], [{'processed': False}]))

    def test_rejects_bad_reference_output(self):
        with self.assertRaises(ValueError):
            decode_output('CORPUS\tinsert\tbad\t00\t00\n')
        with self.assertRaises(ValueError):
            decode_output('CORPUS\tunknown\t0\t00\t00\ttrue\n')
        with self.assertRaises(ValueError):
            decode_output('CORPUS\tnone\t0\t\t\tmaybe\n')


if __name__ == '__main__':
    unittest.main()
