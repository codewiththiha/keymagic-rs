#!/usr/bin/env python3
"""Shows one GTK entry and writes its text to a file on SIGUSR1."""
import signal
import sys

import gi

gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

out_path = sys.argv[1]
window = Gtk.Window(title="CI Probe")
entry = Gtk.Entry()
window.add(entry)
window.connect("destroy", Gtk.main_quit)
window.show_all()
entry.grab_focus()


def dump(*_):
    with open(out_path, "w", encoding="utf-8") as file:
        file.write(entry.get_text())
    return True


GLib.unix_signal_add(GLib.PRIORITY_DEFAULT, signal.SIGUSR1, dump)
Gtk.main()
