#!/usr/bin/env bash
# Builds the standalone `mapcheck` executable, with the whole web app and the
# Rust/WASM core baked into it.
set -euo pipefail

cd "$(dirname "$0")/.."

if [ -d "$HOME/.cargo/bin" ]; then
    PATH="$HOME/.cargo/bin:$PATH"
fi

# Without ./pkg the binary still works, but falls back to the JavaScript
# calculations, so build the wasm first unless it is already there.
if [ ! -f pkg/mapcheck_wasm_bg.wasm ]; then
    echo "No ./pkg yet — building the WebAssembly core first."
    ./scripts/build-wasm.sh
fi

cargo build -p mapcheck-app --release

binary="target/release/mapcheck"
echo
echo "Built $binary ($(du -h "$binary" | cut -f1))"
echo "Run it with:  $binary"
