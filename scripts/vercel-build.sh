#!/bin/sh
# Vercel build: its image has no Rust, so install a minimal toolchain, trunk and wasm-bindgen,
# then build the web app into crates/app/dist (see vercel.json).
set -eu

TRUNK_VERSION=v0.22.2

# Build scripts and proc macros link natively; the image is Amazon Linux 2023.
command -v cc >/dev/null || dnf install -y gcc

curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --target wasm32-unknown-unknown
. "$HOME/.cargo/env"

curl -sSfL "https://github.com/trunk-rs/trunk/releases/download/${TRUNK_VERSION}/trunk-$(uname -m)-unknown-linux-musl.tar.gz" \
    | tar -xz -C "$HOME/.cargo/bin" trunk

# trunk would download wasm-bindgen itself, but its glibc builds can be too new for the
# image; the static musl build in the version Cargo.lock pins always runs.
WASM_BINDGEN_VERSION=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')
WASM_BINDGEN_DIR="wasm-bindgen-${WASM_BINDGEN_VERSION}-$(uname -m)-unknown-linux-musl"
curl -sSfL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${WASM_BINDGEN_VERSION}/${WASM_BINDGEN_DIR}.tar.gz" \
    | tar -xz -C "$HOME/.cargo/bin" --strip-components=1 "${WASM_BINDGEN_DIR}/wasm-bindgen"
wasm-bindgen --version

cd crates/app
trunk build --release
