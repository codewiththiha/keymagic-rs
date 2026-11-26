#!/usr/bin/env bash
# Starts headless sway, binds input-method-v2, and types into a GTK entry.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
export XDG_RUNTIME_DIR="$work/runtime"
mkdir -p -m 700 "$XDG_RUNTIME_DIR"
export WLR_BACKENDS=headless
export WLR_RENDERER=pixman
export WLR_LIBINPUT_NO_DEVICES=1
export LIBSEAT_BACKEND=builtin
trap 'kill $(jobs -p) 2>/dev/null || true; rm -rf "$work"' EXIT

printf 'output HEADLESS-1 resolution 800x600\n' >"$work/sway.conf"
sway -c "$work/sway.conf" >"$work/sway.log" 2>&1 &

socket=""
for _ in $(seq 100); do
    socket=$(find "$XDG_RUNTIME_DIR" -maxdepth 1 -name 'wayland-*' ! -name '*.lock' | head -n 1)
    [ -n "$socket" ] && break
    sleep 0.2
done
if [ -z "$socket" ]; then
    tail -n 60 "$work/sway.log" >&2 || true
    echo "error: sway did not create a Wayland socket" >&2
    exit 1
fi
export WAYLAND_DISPLAY=$(basename "$socket")

timeout 90 "$root/target/debug/keymagic-wayland" >"$work/probe.log" 2>&1 &
probe_pid=$!
for _ in $(seq 100); do
    grep -q "input method bound" "$work/probe.log" 2>/dev/null && break
    sleep 0.2
done
grep -q "input method bound" "$work/probe.log" || {
    cat "$work/probe.log" >&2
    echo "error: the probe did not bind the input method" >&2
    exit 1
}

GDK_BACKEND=wayland GTK_IM_MODULE=wayland timeout 60 \
    python3 "$root/tests/linux/wayland_entry.py" "$work/entry.txt" 15 \
    >"$work/entry.log" 2>&1 || {
    cat "$work/entry.log" >&2
    echo "error: the GTK entry failed" >&2
    exit 1
}

probe_status=0
wait "$probe_pid" || probe_status=$?
cat "$work/probe.log"
if [ "$probe_status" -ne 0 ]; then
    echo "--- sway.log" >&2
    tail -n 60 "$work/sway.log" >&2 || true
    echo "error: the probe exited with $probe_status" >&2
    exit 1
fi
expected=$(printf '\xe1\x80\x81')
actual=$(cat "$work/entry.txt" 2>/dev/null || true)
if [ "$actual" != "$expected" ]; then
    echo "error: the entry holds $(od -An -tx1 <<<"$actual")" >&2
    exit 1
fi
echo "GTK entry received the committed text on $WAYLAND_DISPLAY"
