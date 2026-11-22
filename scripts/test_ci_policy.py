"""Check the self-hosted CI policy probe is configured."""
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


class CiPolicyTests(unittest.TestCase):
    def test_reference_driver_is_formatted_on_self_hosted_runner(self):
        workflow = (ROOT / '.github/workflows/reference-corpus.yml').read_text()
        self.assertIn('runs-on: self-hosted', workflow)
        self.assertIn('rustfmt --edition 2024 --check tools/reference_driver.rs', workflow)
        self.assertIn('push:', workflow)
        self.assertIn("'corpus/**'", workflow)
        self.assertIn('cargo clippy --locked --offline --manifest-path reference-driver/Cargo.toml', workflow)
        self.assertIn('-D warnings -D clippy::all', workflow)
        self.assertIn('cp upstream/Cargo.lock reference-driver/Cargo.lock', workflow)
        self.assertIn('-D clippy::pedantic -D clippy::nursery', workflow)

    def test_clippy_probe_requires_the_requested_lint(self):
        workflow = (ROOT / '.github/workflows/ci.yml').read_text()
        self.assertIn('name: Verify Clippy rejects unwrap', workflow)
        self.assertIn('clippy::unwrap[-_]used', workflow)
        self.assertIn('trap', workflow)
        self.assertIn('file=xtask/src/main.rs', workflow)
        self.assertIn('cargo clippy --locked --package xtask --all-targets', workflow)
        self.assertIn('runs-on: self-hosted', workflow)


if __name__ == '__main__':
    unittest.main()
