"""Test comment diff bases across normal and rewritten history."""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / 'comment_diff_base.py'


class CommentDiffBaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.git('init', '-q')
        self.git('config', 'user.name', 'Test')
        self.git('config', 'user.email', 'test@example.invalid')
        self.root = self.commit('root')
        self.old = self.commit('old')

    def git(self, *args):
        return subprocess.run(['git', *args], cwd=self.repo, check=True,
                              capture_output=True, text=True).stdout.strip()

    def commit(self, text):
        (self.repo / 'file.rs').write_text(text + '\n')
        self.git('add', 'file.rs')
        self.git('commit', '-qm', text)
        return self.git('rev-parse', 'HEAD')

    def base(self, event, before, head):
        return subprocess.run([sys.executable, str(SCRIPT), event, before, head],
                              cwd=self.repo, capture_output=True, text=True)

    def test_fast_forward_uses_before(self):
        head = self.commit('new')
        result = self.base('push', self.old, head)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), self.old)

    def test_rewrite_uses_common_ancestor(self):
        self.git('reset', '--hard', self.root)
        head = self.commit('replacement')
        result = self.base('push', self.old, head)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), self.root)

    def test_missing_base_checks_entire_tree(self):
        result = self.base('push', '0' * 40, self.old)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), self.git('hash-object', '-t', 'tree', '/dev/null'))

    def test_dispatch_uses_head_parent(self):
        result = self.base('workflow_dispatch', self.old, self.old)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), self.root)

    def test_root_commit_uses_empty_tree(self):
        result = self.base('workflow_dispatch', '', self.root)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), self.git('hash-object', '-t', 'tree', '/dev/null'))


if __name__ == '__main__':
    unittest.main()
