#!/bin/sh
# Builds the Worker. Cloudflare's build image has no Rust, so it is
# installed there first; where cargo exists this goes straight to the build.
set -eu

if ! command -v cargo >/dev/null 2>&1; then
  curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.98.0
fi
if [ -f "$HOME/.cargo/env" ]; then
  . "$HOME/.cargo/env"
fi
if command -v rustup >/dev/null 2>&1; then
  rustup target add wasm32-unknown-unknown >/dev/null
fi

command -v worker-build >/dev/null 2>&1 || cargo install -q worker-build
# Stripping removes the table wasm-bindgen's catch wrappers need.
CARGO_PROFILE_RELEASE_STRIP=false worker-build --release
