# KDE Plasma Wayland checks

KWin starts only one input method: the entry chosen under System Settings →
Keyboard → Virtual Keyboard, stored as `[Wayland] InputMethod` in `kwinrc`.
The list is single-select, so IBus and KeyMagic cannot both be active.

## Install

1. Build and install the Arch package on the KDE machine: `makepkg -si`
   in `packaging/arch` (after cloning the repo), or copy the files by hand.
2. Run `keymagic install --user` as your login user. It writes
   `~/.local/share/ibus/component/keymagic.xml` with one engine per enabled keyboard.
3. Make sure `GTK_IM_MODULE` and `QT_IM_MODULE` are unset in the session.

## p6-1: IBus engine (typing check)

1. Open `ibus-setup` and add the KeyMagic engine under Input Method.
2. Open System Settings → Keyboard → Virtual Keyboard and choose
   **IBus Wayland** (`org.freedesktop.IBus.Panel.Wayland.Gtk3.desktop`). Apply.
3. Type in gedit, Firefox and Kate. Record the typed text and which keyboard
   was active. Note any app that does not receive text.

## p6-2: native Wayland input method

1. Open System Settings → Keyboard → Virtual Keyboard and choose **KeyMagic**
   (`keymagic-kwin.desktop`). This replaces IBus Wayland.
2. KWin starts `keymagic-wayland` with `WAYLAND_SOCKET`. Do not run it from
   a terminal: outside KWin the compositor offers no input-method-v1 and the
   probe prints `error: the compositor has no input method protocol`.
3. Type in gedit, Firefox and Kate. Expected: composed text commits; keys
   the engine does not use pass through unchanged.

## What to report

- The Plasma version (`plasmashell --version`) and the selected entry.
- Each app and whether typing worked.
- Any error text from `journalctl --user -b` for `keymagic` or `ibus`.

## Not yet covered

- kimpanel panel widget (p6-2 step 6).
- Surrounding-text deletes and direct-mode fallback.
- The KWin-side selection is not set by the package; the user chooses it.
