#!/bin/bash
# Builds the browser version into web/dist (static files; any web server will do).
# Needs: rustup target add wasm32-unknown-unknown; cargo install wasm-bindgen-cli --version 0.2.106
set -euo pipefail
cd "$(dirname "$0")"
v_main=$(sed -n 's/^version = "\(.*\)"/\1/p' ../Cargo.toml | head -1)
v_web=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
if [ "$v_main" != "$v_web" ]; then
  echo "web/Cargo.toml has version $v_web, ../Cargo.toml has $v_main: please set both to the same" >&2
  exit 1
fi
cargo build --release --target wasm32-unknown-unknown
rm -rf dist pkg
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/release/hla_wgs.wasm
mkdir -p dist/data
cp www/* dist/
cp -r pkg dist/pkg
cp ../data/freq_1000g_2field.json ../data/panel_mask_3.66.bed dist/data/
echo "web/dist ready (hla-wgs $v_web): $(du -sh dist | cut -f1)"
