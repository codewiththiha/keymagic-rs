# Linux manual testing — pending

Status: **not run**. Record date, branch/SHA, package version, OS/desktop/compositor, application, keyboard, exact actions and result for each row. CI compilation is not manual typing evidence.

| Environment | App | Checks | Result / evidence |
| --- | --- | --- | --- |
| GNOME Wayland via IBus | GTK editor, Firefox | source appears, type, preedit/commit, switch, focus | Not run |
| X11 via IBus | GTK and Qt apps | source, type, switch, environment variables | Not run |
| KDE Plasma 6 Wayland | GTK and Qt apps | native IM registration, kimpanel, switching | Not run |
| Sway/Hyprland and supported v2 compositor | GTK and Qt apps | preedit, forwarding, surrounding text | Not run |
| Arch, Debian, Fedora | installer | install, removal, bundled keyboards, doctor | Not run |
| Fcitx5 (optional) | Konsole, Kate | shim registration and typing | Not run |
