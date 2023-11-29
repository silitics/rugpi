#!/usr/bin/env bash

set -euo pipefail

cargo chef cook --release --bin rugpi-bakery --recipe-path recipe.json

# Prepare to build binaries for both, 32-bit and 64-bit Raspberry Pi.
cargo chef cook --release --target arm-unknown-linux-musleabihf --recipe-path recipe.json
cargo chef cook --release --target aarch64-unknown-linux-musl --recipe-path recipe.json
