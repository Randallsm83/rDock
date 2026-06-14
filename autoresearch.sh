#!/usr/bin/env bash
# Canonical autoresearch benchmark entrypoint for rdock.
#
# Measures the per-frame cost of the software renderer's hot path
# (`Renderer::render`): magnification animation, bicubic icon scaling,
# alpha blending, anti-aliased rounded background, reflections.
#
# The workload is deterministic: a fixed dock layout, a procedurally generated
# icon fixture, and a fixed synthetic magnification sweep over a fixed number of
# frames. Primary metric is microseconds per frame (lower is better), reported
# as the minimum over many timed batches to suppress scheduler noise.
set -euo pipefail
cd "$(dirname "$0")"

# Resolve cargo: it may not be on PATH in non-interactive shells.
CARGO="${CARGO:-}"
if [ -z "$CARGO" ] || ! command -v "$CARGO" >/dev/null 2>&1; then
    if command -v cargo >/dev/null 2>&1; then
        CARGO="cargo"
    else
        for c in "$HOME/.cargo/bin/cargo.exe" "$HOME/.cargo/bin/cargo" \
                 "${USERPROFILE:-}/.cargo/bin/cargo.exe" \
                 "/c/Users/ranmil/.cargo/bin/cargo.exe" \
                 "C:/Users/ranmil/.cargo/bin/cargo.exe" \
                 "C:/Users/ranmil/.local/share/mise/installs/rust/stable/cargo.exe" \
                 "/mnt/c/Users/ranmil/.cargo/bin/cargo.exe" \
                 "/mnt/c/Users/ranmil/.local/share/mise/installs/rust/stable/cargo.exe"; do
            if [ -x "$c" ]; then CARGO="$c"; break; fi
        done
    fi
fi
if [ -z "$CARGO" ] || { [ "$CARGO" != "cargo" ] && [ ! -x "$CARGO" ]; }; then
    echo "cargo not found (set \$CARGO to its path)" >&2
    exit 127
fi

# Build the bench binary with production-grade optimization (release profile).
# Build logs go to stderr so stdout carries only METRIC lines.
"$CARGO" build --release --features bench --bin render_bench >&2

BIN="target/release/render_bench"
if [ -f "${BIN}.exe" ]; then
    BIN="${BIN}.exe"
fi

if [ ! -f "$BIN" ]; then
    echo "render_bench binary not found at $BIN" >&2
    exit 1
fi

exec "$BIN"
