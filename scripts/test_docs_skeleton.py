"""Static documentation-presence checks; not a substitute for docs builds."""
from pathlib import Path
import unittest

from test_workspace_skeleton import PACKAGES

ROOT = Path(__file__).resolve().parent.parent
REQUIRED = (
    'AGENTS.md', 'CONTRIBUTING.md', 'LICENSE', 'docs/ARCHITECTURE.md',
    'docs/KMS_FORMAT.md', 'docs/KM2_FORMAT.md', 'docs/ENGINE_LOGIC.md',
    'docs/VIRTUAL_KEYS.md', 'docs/ADR/0001-engine-first.md',
    'docs/TESTING_LINUX.md', 'docs/TESTING_WINDOWS.md', 'docs/TESTING_MACOS.md',
)


class DocumentationSkeletonTests(unittest.TestCase):
    def test_required_documents_exist_and_are_nonempty(self):
        for path in REQUIRED:
            with self.subTest(path=path):
                item = ROOT / path
                self.assertTrue(item.is_file(), path)
                self.assertGreater(item.stat().st_size, 50, path)

    def test_reference_documents_record_provenance(self):
        for name in ('KMS_FORMAT.md', 'KM2_FORMAT.md', 'ENGINE_LOGIC.md'):
            with self.subTest(name=name):
                text = (ROOT / 'docs' / name).read_text()
                self.assertIn('thantthet/keymagic-3', text)
                self.assertIn('2db30dc8e8a5f091d1434a2bc1702b29cf113804', text)

    def test_each_package_has_a_readme(self):
        for path in PACKAGES:
            with self.subTest(path=path):
                text = (ROOT / path / 'README.md').read_text()
                self.assertIn('Placeholder', text)
                self.assertIn('no-unsafe', text)

    def test_owner_runner_and_safety_rules_documented(self):
        text = (ROOT / 'CONTRIBUTING.md').read_text()
        for expected in ('kt007441@gmail.com', 'self-hosted', 'unsafe', '50'):
            self.assertIn(expected, text)


if __name__ == '__main__':
    unittest.main()
