#!/usr/bin/env bash
# Starts headless sway, grabs the keyboard, and types through the engine.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
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
export XDG_CACHE_HOME="$work/cache" XDG_RUNTIME_DIR="$work/runtime"
export HOME="$work/home" PATH="$root/target/debug:$PATH"
mkdir -p "$XDG_RUNTIME_DIR" "$HOME" "$XDG_CONFIG_HOME/keymagic/keyboards"
chmod 700 "$XDG_RUNTIME_DIR"

keymagic compile "$root/tests/linux/probe2.kms" \
    --output "$XDG_CONFIG_HOME/keymagic/keyboards/probe2.km2"
cat > "$XDG_CONFIG_HOME/keymagic/config.toml" <<'TOML'
[keyboards]
active = "probe2"

[[keyboards.installed]]
id = "probe2"
name = "Probe Two"
filename = "probe2.km2"
TOML

export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
export LIBSEAT_BACKEND=builtin
printf 'output HEADLESS-1 resolution 800x600\n' >"$work/sway.conf"
sway -c "$work/sway.conf" >"$work/sway.log" 2>&1 &

socket=""
for _ in $(seq 100); do
    socket=$(find "$XDG_RUNTIME_DIR" -maxdepth 1 -name 'wayland-*' ! -name '*.lock' | head -n 1)
    [ -n "$socket" ] && break
    sleep 0.2
done
[ -n "$socket" ] || fail "sway did not create a Wayland socket"
export WAYLAND_DISPLAY=$(basename "$socket")

wait_for() {
    for _ in $(seq $(($2 * 5))); do
        grep -q "$1" "$work/probe.log" 2>/dev/null && return 0
        sleep 0.2
    done
    return 1
}

timeout 120 "$root/target/debug/keymagic-wayland" >"$work/probe.log" 2>&1 &
wait_for "input method bound" 30 || fail "the probe did not bind the input method"

GDK_BACKEND=wayland GTK_IM_MODULE=wayland timeout 90 \
    python3 "$root/tests/linux/wayland_entry.py" "$work/entry.txt" 25 \
    >"$work/entry.log" 2>&1 &
entry_pid=$!
wait_for "keyboard grab ready" 60 || fail "the entry did not activate the input method"

# x passes through the virtual keyboard; k composes to the probe glyph.
wtype xk
wtype -k Return
wait "$entry_pid" || fail "the GTK entry exited with an error"

expected=$(printf 'x\xe1\x80\x81')
actual=$(cat "$work/entry.txt" 2>/dev/null || true)
if [ "$actual" != "$expected" ]; then
    fail "the entry holds $(od -An -tx1 <<<"$actual") from $(wc -c <"$work/entry.txt" 2>/dev/null || echo 0) bytes"
fi
echo "GTK entry received x and the engine text through the grab on $WAYLAND_DISPLAY"
