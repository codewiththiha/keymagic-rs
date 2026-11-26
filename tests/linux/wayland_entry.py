"""GTK 3 entry under Wayland: saves its text to a file after a delay."""
import sys

import gi

gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk  # noqa: E402

OUTPUT = sys.argv[1]
DELAY_SECONDS = int(sys.argv[2]) if len(sys.argv) > 2 else 15


def save_and_quit(entry):
    with open(OUTPUT, "w", encoding="utf-8") as handle:
        handle.write(entry.get_text())
    Gtk.main_quit()
    return False


window = Gtk.Window(title="KeyMagic Wayland probe")
entry = Gtk.Entry()
window.add(entry)
window.connect("destroy", Gtk.main_quit)
window.show_all()
entry.grab_focus()
GLib.timeout_add_seconds(DELAY_SECONDS, save_and_quit, entry)
Gtk.main()
