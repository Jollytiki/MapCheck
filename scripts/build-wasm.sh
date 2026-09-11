#!/usr/bin/env bash
# Builds the Rust calculation core to WebAssembly and drops it in ./pkg,
# where wasm-bridge.js looks for it.
#
# Prerequisites (one time):
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-pack
set -euo pipefail

cd "$(dirname "$0")/.."

# wasm-pack installs to ~/.cargo/bin, which is not always on PATH.
if [ -d "$HOME/.cargo/bin" ]; then
    PATH="$HOME/.cargo/bin:$PATH"
fi

if ! command -v wasm-pack >/dev/null 2>&1; then
    echo "wasm-pack is not installed. Install it with:" >&2
    echo "  cargo install wasm-pack" >&2
    exit 1
fi

# `--print target-libdir` reports where the target *would* live, so the
# directory has to be checked to know whether it is actually installed.
has_wasm_target() {
    local libdir
    libdir=$(rustc --print target-libdir --target wasm32-unknown-unknown 2>/dev/null) || return 1
    # On Windows rustc reports a backslash path, which bash does not treat as
    # separators, so the test would fail on a target that is actually present.
    [ -d "${libdir//\\//}" ]
}

# A Homebrew rustc has no wasm32 target and cannot be given one, so prefer a
# rustup toolchain when both are installed.
if ! has_wasm_target && command -v rustup >/dev/null 2>&1; then
    if toolchain=$(rustup which rustc 2>/dev/null); then
        PATH="$(dirname "${toolchain//\\//}"):$PATH"
    fi
fi

if ! has_wasm_target; then
    echo "The wasm32-unknown-unknown target is not available. Add it with:" >&2
    echo "  rustup target add wasm32-unknown-unknown" >&2
    echo "(If rustc came from Homebrew there is no rustup to add targets with;" >&2
    echo " install one with: brew install rustup && rustup default stable)" >&2
    exit 1
fi

# Some rustup toolchains ship rust-lld with an rpath that misses the bundled
# libLLVM.dylib one directory up. Point the loader at it rather than making the
# user reinstall the toolchain.
toolchain_lib="$(dirname "$(dirname "$(command -v rustc)")")/lib"
if [ -f "$toolchain_lib/libLLVM.dylib" ]; then
    export DYLD_FALLBACK_LIBRARY_PATH="$toolchain_lib${DYLD_FALLBACK_LIBRARY_PATH:+:$DYLD_FALLBACK_LIBRARY_PATH}"
fi

PROFILE="${1:---release}"
wasm-pack build crates/mapcheck-wasm \
    --target web \
    --out-dir ../../pkg \
    --out-name mapcheck_wasm \
    "$PROFILE"

echo
echo "Built ./pkg — reload the app and the console will report:"
echo "  MapCheck: calculations running on the Rust/WASM core"
