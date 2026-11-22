import unittest
from check_commit_messages import problems


class CommitPolicyTests(unittest.TestCase):
    def test_valid(self):
        for message in ('feat: add keys', 'fix(engine): handle input',
                        'feat!: change API', 'docs: မြန်မာ', 'chore: ' + 'a' * 43,
                        'docs: first line\n'):
            with self.subTest(message=message):
                self.assertEqual(problems(message), [])

    def test_invalid(self):
        for message in ('', '   ', 'feat: ' + 'a' * 45, 'feat: title\n\nbody',
                        'plain message', 'feat:add keys', 'feat: ',
                        'feat(): add keys', 'hello\rworld'):
            with self.subTest(message=message):
                self.assertTrue(problems(message))


if __name__ == '__main__':
    unittest.main()
