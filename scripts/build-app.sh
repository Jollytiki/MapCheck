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

# Whether the rustc on PATH can build for a target. `--print target-libdir`
# names where it would live whether or not it is installed, so the directory
# has to be checked. Windows paths come back with backslashes.
has_target() {
    local libdir
    libdir=$(rustc --print target-libdir --target "$1" 2>/dev/null) || return 1
    [ -d "${libdir//\\//}" ]
}

# A release build for macOS covers both architectures, so one download runs
# on Apple Silicon and Intel alike. cargo cannot emit a universal binary
# itself, so the two are built separately and merged.
if [ -n "${MAPCHECK_UNIVERSAL:-}" ]; then
    # A Homebrew rustc has only the host target and cannot be given another,
    # so fall back to a rustup toolchain the way build-wasm.sh does.
    for target in aarch64-apple-darwin x86_64-apple-darwin; do
        if ! has_target "$target" && command -v rustup >/dev/null 2>&1; then
            if toolchain=$(rustup which rustc 2>/dev/null); then
                PATH="$(dirname "${toolchain//\\//}"):$PATH"
            fi
        fi
    done

    for target in aarch64-apple-darwin x86_64-apple-darwin; do
        if ! has_target "$target"; then
            echo "The $target target is not installed. Add it with:" >&2
            echo "  rustup target add $target" >&2
            exit 1
        fi
    done

    cargo build --locked -p mapcheck-app --release --target aarch64-apple-darwin
    cargo build --locked -p mapcheck-app --release --target x86_64-apple-darwin
    mkdir -p target/release
    lipo -create -output target/release/mapcheck \
        target/aarch64-apple-darwin/release/mapcheck \
        target/x86_64-apple-darwin/release/mapcheck
    echo "Merged a universal binary:"
    lipo -archs target/release/mapcheck | sed 's/^/  /'
else
    cargo build -p mapcheck-app --release
fi

binary="target/release/mapcheck"
echo
echo "Built $binary ($(du -h "$binary" | cut -f1))"
echo "Run it with:  $binary"
