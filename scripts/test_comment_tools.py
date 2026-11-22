"""Tests for source comment rules and diff budgets."""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / 'tools'))
from comment_check import check_source
from comment_budget import count_added


class CommentCheckTests(unittest.TestCase):
    def test_module_doc_up_to_five_lines(self):
        self.assertEqual(check_source('//! one\n//! two\nfn main() {}\n', '.rs'), [])
        self.assertTrue(check_source('//! one\n' * 6 + 'fn main() {}\n', '.rs'))

    def test_single_comments_and_doc_lines(self):
        self.assertEqual(check_source('//! module\n\n/// Item contract.\npub fn f() {}\n', '.rs'), [])
        self.assertTrue(check_source('fn f() {}\n// one\n// two\n', '.rs'))
        self.assertTrue(check_source('/// first\n/// second\nfn f() {}\n', '.rs'))
        self.assertTrue(check_source('fn f() {} // ' + 'word ' * 16 + '\n', '.rs'))
        self.assertTrue(check_source('fn f() {} // ' + 'x' * 81 + '\n', '.rs'))

    def test_literals_do_not_create_comments(self):
        source = '//! module\nlet a = "https://example.test";\nlet b = r#"// not a comment"#;\n'
        self.assertEqual(check_source(source, '.rs'), [])
        self.assertEqual(check_source('x = "# not a comment"\n', '.py'), [])

    def test_forbidden_and_block_comments(self):
        self.assertTrue(check_source('// TODO fix\n', '.rs'))
        self.assertTrue(check_source('/* first\nsecond */\n', '.rs'))
        self.assertTrue(check_source('# first\n# second\n', '.py'))
        self.assertTrue(check_source('# comment-check: allow\n', '.py'))

    def test_module_docstring_limit(self):
        self.assertEqual(check_source('"""One line."""\nx = 1\n', '.py'), [])
        self.assertTrue(check_source('"""a\nb\nc\nd\ne\nf"""\nx = 1\n', '.py'))


class CommentBudgetTests(unittest.TestCase):
    def test_counts_only_added_code_and_ordinary_comments(self):
        old = '//! module\nfn main() {}\n'
        new = '//! module\n// constraint\nfn main() {\n    let a = 1;\n}\n'
        code, comments = count_added(old, new, '.rs')
        self.assertEqual((code, comments), (3, 1))

    def test_docs_do_not_spend_ordinary_comment_budget(self):
        self.assertEqual(count_added('', '//! module\n/// contract\nfn f() {}\n', '.rs'), (1, 0))

    def test_cli_rejects_excess_comments(self):
        script = Path(__file__).resolve().parent.parent / 'tools/comment_budget.py'
        with tempfile.TemporaryDirectory() as directory:
            def run(*args):
                return subprocess.run(args, cwd=directory, capture_output=True, text=True)

            self.assertEqual(run('git', 'init', '-q').returncode, 0)
            path = Path(directory) / 'sample.rs'
            path.write_text('fn f() {}\n')
            self.assertEqual(run('git', 'add', 'sample.rs').returncode, 0)
            self.assertEqual(run('git', '-c', 'user.name=Test', '-c', 'user.email=t@example.invalid',
                                 'commit', '-qm', 'Initial').returncode, 0)
            path.write_text('fn f() {}\n// one\n\n// two\n\n// three\n')
            result = run(sys.executable, str(script), '--base', 'HEAD', 'sample.rs')
            self.assertEqual(result.returncode, 1)
            self.assertIn('exceed', result.stderr)


if __name__ == '__main__':
    unittest.main()
