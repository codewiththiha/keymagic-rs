import unittest
from check_commit_authors import author_problem


class CommitAuthorTests(unittest.TestCase):
    def test_owner_email_allowed(self):
        self.assertIsNone(author_problem('kt007441@gmail.com'))

    def test_other_email_rejected(self):
        self.assertIsNotNone(author_problem('other@example.com'))
        self.assertIsNotNone(author_problem(''))


if __name__ == '__main__':
    unittest.main()
