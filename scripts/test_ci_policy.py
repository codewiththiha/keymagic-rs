"""Check the self-hosted CI policy probe is configured."""
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


class CiPolicyTests(unittest.TestCase):
    def test_clippy_probe_requires_the_requested_lint(self):
        workflow = (ROOT / '.github/workflows/ci.yml').read_text()
        self.assertIn('name: Verify Clippy rejects unwrap', workflow)
        self.assertIn('clippy::unwrap[-_]used', workflow)
        self.assertIn('trap', workflow)
        self.assertIn('runs-on: self-hosted', workflow)


if __name__ == '__main__':
    unittest.main()
