#!/bin/sh
# Vercel build: install a current Rust toolchain with the WebAssembly target, then build the
# web app into web/dist (see vercel.json). web/scripts/build-wasm.sh fetches wasm-bindgen.
set -eu

# Build scripts and proc macros link natively; the image is Amazon Linux 2023.
command -v cc >/dev/null || dnf install -y gcc

# The image ships an old rustup toolchain in /rust; dependencies need a newer one either way.
if ! command -v rustup >/dev/null; then
    curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
    . "$HOME/.cargo/env"
fi
rustup toolchain install stable --profile minimal --target wasm32-unknown-unknown
rustup default stable

cd web
npm ci
npm run build
