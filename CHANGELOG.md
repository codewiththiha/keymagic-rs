# Changelog

## Unreleased

- Keyboards: cards show the icon, hotkey and active choice. Drag a card by its grip to reorder. The options menu sets the hotkey, renames, shows the file, moves, and removes after confirmation. Filter and Open folder added. A banner offers to install the bundled Pyidaungsu MM keyboard when it is missing.
- Tester: a diagnostics pane shows the last key, whether it was handled, the output, the composing text, and the rule and source line behind it. Record case saves the typed input and output to `<keyboard>.cases.toml`. The Tester reloads after each compile.
- Editor: KMS syntax highlighting; the layout preview shows Shift and AltGr layers while those keys are held; the glyph picker includes Myanmar Extended-A letters and the assigned Extended-B letters.
- Settings: each integration panel has a status dot. A missing Windows or macOS integration shows an Open settings button for the system input settings. A Keyboard HUD switch turns the keyboard-switch overlay on or off. On Linux, cards for IBus, Wayland, KDE Plasma and Fcitx5 show whether each is running, with a Fix button that asks before it runs a restart command as your user.
- Tray: the icon is accent while a keyboard is active and grey otherwise.
- About: this changelog, an update check with a Remind me later option, and the third-party licence list.
