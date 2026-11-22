"""Static checks for the phase 00 workspace layout; these do not replace Cargo."""
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parent.parent
PACKAGES = {
    'crates/keymagic-types': ('keymagic-types', 'lib'),
    'crates/keymagic-km2': ('keymagic-km2', 'lib'),
    'crates/keymagic-engine': ('keymagic-engine', 'lib'),
    'crates/keymagic-kms': ('keymagic-kms', 'lib'),
    'crates/keymagic-ffi': ('keymagic-ffi', 'cdylib'),
    'crates/keymagic-cli': ('keymagic-cli', 'bin'),
    'crates/keymagic-host': ('keymagic-host', 'lib'),
    'crates/keymagic-gui': ('keymagic-gui', 'bin'),
    'platform/windows/keymagic-tsf': ('keymagic-windows-tsf', 'cdylib'),
    'platform/windows/keymagic-tray': ('keymagic-windows-tray', 'bin'),
    'platform/macos/keymagic-imk': ('keymagic-macos-imk', 'bin'),
    'platform/linux/keymagic-ibus': ('keymagic-linux-ibus', 'bin'),
    'platform/linux/keymagic-wayland': ('keymagic-linux-wayland', 'bin'),
    'platform/linux/keymagic-fcitx5': ('keymagic-linux-fcitx5', 'cdylib'),
    'xtask': ('xtask', 'bin'),
}


def read_toml(path):
    with path.open('rb') as file:
        return tomllib.load(file)


class WorkspaceSkeletonTests(unittest.TestCase):
    def test_workspace_configuration(self):
        root = read_toml(ROOT / 'Cargo.toml')['workspace']
        self.assertEqual(root['members'], ['crates/*', 'platform/*/*', 'xtask'])
        self.assertEqual(root['resolver'], '2')
        self.assertEqual(root['package']['edition'], '2021')
        self.assertEqual(root['lints']['clippy']['all']['level'], 'deny')
        self.assertEqual(root['lints']['clippy']['unwrap_used'], 'warn')

    def test_all_packages_have_correct_stubs(self):
        for path, (name, kind) in PACKAGES.items():
            with self.subTest(package=name):
                directory = ROOT / path
                manifest = read_toml(directory / 'Cargo.toml')
                self.assertEqual(manifest['package']['name'], name)
                self.assertTrue(manifest['lints']['workspace'])
                source = directory / 'src' / ('main.rs' if kind == 'bin' else 'lib.rs')
                self.assertTrue(source.is_file())
                self.assertIn('//!', source.read_text())
                if kind == 'cdylib':
                    self.assertIn('cdylib', manifest['lib']['crate-type'])
                if path.startswith('crates/') and kind != 'cdylib':
                    self.assertIn('#![forbid(unsafe_code)]', source.read_text())

    def test_member_globs_and_platform_binary_fallbacks(self):
        workspace = read_toml(ROOT / 'Cargo.toml')['workspace']
        excluded = set(workspace.get('exclude', []))
        discovered = set()
        for pattern in workspace['members']:
            for path in ROOT.glob(pattern):
                relative = path.relative_to(ROOT).as_posix()
                if relative not in excluded:
                    self.assertTrue((path / 'Cargo.toml').is_file(), relative)
                    discovered.add(relative)
        self.assertEqual(discovered, set(PACKAGES))
        for path, (_, kind) in PACKAGES.items():
            if not path.startswith('platform/') or kind != 'bin':
                continue
            target = {'windows': 'windows', 'macos': 'macos', 'linux': 'linux'}[path.split('/')[1]]
            source = (ROOT / path / 'src/main.rs').read_text()
            self.assertIn(f'#[cfg(target_os = "{target}")]', source)
            self.assertIn(f'#[cfg(not(target_os = "{target}"))]', source)

    def test_toolchain_and_hygiene(self):
        toolchain = read_toml(ROOT / 'rust-toolchain.toml')['toolchain']
        self.assertEqual(toolchain['channel'], 'stable')
        self.assertTrue({'rustfmt', 'clippy'} <= set(toolchain['components']))
        self.assertEqual(read_toml(ROOT / 'rustfmt.toml')['max_width'], 110)
        self.assertTrue((ROOT / '.editorconfig').is_file())
        self.assertIn('licenses', read_toml(ROOT / 'deny.toml'))


if __name__ == '__main__':
    unittest.main()
