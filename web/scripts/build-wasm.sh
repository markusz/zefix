#!/bin/sh
# Builds crates/wasm for the browser into web/src/wasm (generated, not committed).
# Uses the wasm-bindgen CLI in exactly the version Cargo.lock pins, fetched once into
# target/tools, since the generated JavaScript must match the crate.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT"

VERSION=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')
case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) PLATFORM=aarch64-apple-darwin ;;
    Darwin-x86_64) PLATFORM=x86_64-apple-darwin ;;
    Linux-x86_64) PLATFORM=x86_64-unknown-linux-musl ;;
    Linux-aarch64) PLATFORM=aarch64-unknown-linux-musl ;;
    *) echo "Keine wasm-bindgen-Version für $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac
TOOLS="target/tools/wasm-bindgen-$VERSION"
if [ ! -x "$TOOLS/wasm-bindgen" ]; then
    mkdir -p "$TOOLS"
    NAME="wasm-bindgen-$VERSION-$PLATFORM"
    curl -sSfL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$VERSION/$NAME.tar.gz" \
        | tar -xz -C "$TOOLS" --strip-components=1 "$NAME/wasm-bindgen"
fi

cargo build -p zefix-wasm --release --target wasm32-unknown-unknown
"$TOOLS/wasm-bindgen" --target web --out-dir web/src/wasm \
    target/wasm32-unknown-unknown/release/zefix_wasm.wasm
