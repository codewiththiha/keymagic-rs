"""Check the implemented workspace without invoking Cargo."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parent.parent
PACKAGES = {'xtask': ('xtask', 'bin'), 'crates/keymagic-types': ('keymagic-types', 'lib'), 'crates/keymagic-km2': ('keymagic-km2', 'lib'), 'crates/keymagic-engine': ('keymagic-engine', 'lib')}


def read_toml(path):
    with path.open('rb') as file:
        return tomllib.load(file)


class WorkspaceSkeletonTests(unittest.TestCase):
    def test_workspace_configuration(self):
        root = read_toml(ROOT / 'Cargo.toml')['workspace']
        self.assertEqual(root['members'], ['crates/keymagic-cli', 'crates/keymagic-engine', 'crates/keymagic-kms', 'crates/keymagic-km2', 'crates/keymagic-types', 'xtask'])
        self.assertEqual(root['resolver'], '3')
        self.assertEqual(root['package']['edition'], '2024')
        self.assertEqual(root['lints']['rust']['unsafe_code'], 'forbid')
        for group in ('all', 'pedantic', 'nursery'):
            self.assertEqual(root['lints']['clippy'][group]['level'], 'deny')
        self.assertEqual(root['lints']['clippy']['unwrap_used'], 'deny')
        self.assertEqual(root['lints']['clippy']['expect_used'], 'deny')

    def test_only_implemented_package_is_present(self):
        manifest = read_toml(ROOT / 'xtask/Cargo.toml')
        self.assertEqual(manifest['package']['name'], 'xtask')
        self.assertTrue(manifest['lints']['workspace'])
        source = (ROOT / 'xtask/src/main.rs').read_text()
        self.assertIn('#![forbid(unsafe_code)]', source)
        self.assertIn('corpus-sync', source)
        self.assertTrue((ROOT / 'crates/keymagic-types/Cargo.toml').is_file())
        packages = read_toml(ROOT / 'Cargo.lock')['package']
        names = {package['name'] for package in packages}
        self.assertTrue({'keymagic-km2', 'keymagic-types', 'xtask'} <= names)

    def test_toolchain_and_hygiene(self):
        toolchain = read_toml(ROOT / 'rust-toolchain.toml')['toolchain']
        self.assertEqual(toolchain['channel'], 'stable')
        self.assertTrue({'rustfmt', 'clippy'} <= set(toolchain['components']))
        self.assertEqual(read_toml(ROOT / 'rustfmt.toml')['max_width'], 100)
        self.assertTrue((ROOT / '.editorconfig').is_file())
        self.assertIn('licenses', read_toml(ROOT / 'deny.toml'))


if __name__ == '__main__':
    unittest.main()
