import unittest
from check_commit_messages import problems


class CommitPolicyTests(unittest.TestCase):
    def test_valid(self):
        self.assertEqual(problems('a' * 50), [])
        self.assertEqual(problems('မြန်မာ'), [])
        self.assertEqual(problems('subject\n'), [])

    def test_invalid(self):
        for message in ('', '   ', 'a' * 51, 'subject\n\nbody', 'hello\rworld'):
            with self.subTest(message=message):
                self.assertTrue(problems(message))


if __name__ == '__main__':
    unittest.main()
