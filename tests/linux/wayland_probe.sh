#!/usr/bin/env bash
# Starts headless sway and checks that the Wayland probe binds input-method-v2.
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
    echo "--- sway.log" >&2
    tail -n 60 "$work/sway.log" >&2 || true
    echo "error: sway did not create a Wayland socket" >&2
    exit 1
fi
export WAYLAND_DISPLAY=$(basename "$socket")

cargo_bin="$root/target/debug/keymagic-wayland"
"$cargo_bin" >"$work/probe.log" 2>&1 || {
    echo "--- probe.log" >&2
    cat "$work/probe.log" >&2
    echo "--- sway.log" >&2
    tail -n 60 "$work/sway.log" >&2 || true
    echo "error: the Wayland probe failed" >&2
    exit 1
}
cat "$work/probe.log"
grep -q "input method bound" "$work/probe.log" || {
    echo "error: the probe did not bind the input method" >&2
    exit 1
}
echo "sway bound input-method-v2 on $WAYLAND_DISPLAY"
