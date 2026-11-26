#!/usr/bin/env bash
# Types through IBus and the KeyMagic engine into a GTK entry, then compares.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
if [ -z "${DBUS_SESSION_BUS_ADDRESS:-}" ]; then
    exec dbus-run-session -- bash "$0" "$@"
fi

work=$(mktemp -d)
fail() {
    echo "error: $1" >&2
    for log in "$work"/*.log; do
        echo "--- $(basename "$log")" >&2
        tail -n 60 "$log" >&2 || true
    done
    exit 1
}
trap 'kill $(jobs -p) 2>/dev/null || true; rm -rf "$work"' EXIT

export XDG_CONFIG_HOME="$work/config" XDG_DATA_HOME="$work/data"
export XDG_CACHE_HOME="$work/cache" XDG_RUNTIME_DIR="$work/run"
export HOME="$work/home" PATH="$root/target/debug:$PATH"
export DISPLAY=:99 GTK_IM_MODULE=ibus GDK_BACKEND=x11
mkdir -p "$XDG_RUNTIME_DIR" "$HOME" "$XDG_CONFIG_HOME/keymagic/keyboards"
chmod 700 "$XDG_RUNTIME_DIR"
# D-Bus auto-activation must see the same paths, or it starts IBus without the component.
dbus-update-activation-environment XDG_CONFIG_HOME XDG_DATA_HOME XDG_CACHE_HOME \
    XDG_RUNTIME_DIR HOME PATH DISPLAY

keymagic compile "$root/tests/linux/probe.kms" \
    --output "$XDG_CONFIG_HOME/keymagic/keyboards/probe.km2"
keymagic compile "$root/tests/linux/probe2.kms" \
    --output "$XDG_CONFIG_HOME/keymagic/keyboards/probe2.km2"
cat > "$XDG_CONFIG_HOME/keymagic/config.toml" <<'TOML'
[keyboards]
active = "probe"

[[keyboards.installed]]
id = "probe"
name = "Probe"
filename = "probe.km2"

[[keyboards.installed]]
id = "probe2"
name = "Probe Two"
filename = "probe2.km2"
TOML
keymagic install --user

Xvfb :99 -screen 0 1024x768x24 >"$work/xvfb.log" 2>&1 &
sleep 2

# The daemon scans only the system component directory unless told otherwise.
export IBUS_COMPONENT_PATH="$XDG_DATA_HOME/ibus/component:/usr/share/ibus/component"
ibus-daemon -rx --panel disable >>"$work/ibus.log" 2>&1 &
published=""
for _ in $(seq 300); do
    if ls "$XDG_CONFIG_HOME"/ibus/bus/*-unix-99 >/dev/null 2>&1; then
        published=1
        break
    fi
    sleep 0.2
done
[ -n "$published" ] || fail "ibus-daemon did not publish its bus address"
sleep 1

ibus_addr=$(sed -n 's/^IBUS_ADDRESS=//p' "$XDG_CONFIG_HOME"/ibus/bus/*-unix-99 | head -n 1)
dbus-monitor --address "$ibus_addr" >"$work/monitor.log" 2>&1 &
sleep 1

# The client exit status is unreliable here, so the selection is read back below.
timeout 20 ibus engine keymagic:probe >"$work/select.log" 2>&1 || true
[ "$(timeout 10 ibus engine 2>/dev/null || true)" = "keymagic:probe" ] \
    || fail "cannot select keymagic:probe"

python3 "$root/tests/linux/entry_probe.py" "$work/out.txt" >"$work/probe.log" 2>&1 &
probe=$!
window=$(timeout 30 xdotool search --sync --name "CI Probe" | head -n 1) \
    || fail "no CI Probe window"
xdotool windowfocus --sync "$window"
sleep 1
# The keyboard menu is registered when the engine gains focus.
grep -q "member=RegisterProperties" "$work/monitor.log" \
    || fail "no keyboard menu was registered"
# Choose the second keyboard through the menu, as the panel does.
menu_signal=$(grep "^signal sender=:.*member=RegisterProperties" "$work/monitor.log" | head -n 1)
engine_name=$(printf '%s\n' "$menu_signal" | sed -n 's/^signal sender=\(:[^ ]*\) .*/\1/p')
engine_path=$(printf '%s\n' "$menu_signal" | sed -n 's/.*path=\([^;]*\);.*/\1/p')
[ -n "$engine_name" ] && [ -n "$engine_path" ] || fail "cannot find the engine in the menu signal"
dbus-send --address="$ibus_addr" --type=method_call --dest="$engine_name" \
    "$engine_path" org.freedesktop.IBus.Engine.PropertyActivate \
    string:keymagic:probe2 uint32:1 || fail "the menu did not switch keyboards"
sleep 1
xdotool type --delay 300 kk
# Composed text stays in the preedit until Return commits it (Trigger::Enter).
sleep 1
xdotool key --delay 300 Return

expected=$(printf '\xe1\x80\x81\xe1\x80\x81')
for _ in $(seq 50); do
    kill -USR1 "$probe" 2>/dev/null || true
    if [ -f "$work/out.txt" ] && [ "$(cat "$work/out.txt")" = "$expected" ]; then
        echo "typed text matches after the menu switch: kk -> ခခ"
        exit 0
    fi
    sleep 0.2
done
grep -E "member=(ProcessKeyEvent|CommitText|FocusIn|CreateInputContext|SetCapabilities|Enable)|error_name" \
    "$work/monitor.log" | tail -n 40 >&2 || true
fail "typed text did not match; got $(od -An -tx1 "$work/out.txt" 2>/dev/null)"
