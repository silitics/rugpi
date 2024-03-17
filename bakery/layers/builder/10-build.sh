#!/usr/bin/env bash

set -euo pipefail

cargo build --release --bin rugpi-bakery

# Build binaries for both, 32-bit and 64-bit Raspberry Pi.
cargo build --release --target arm-unknown-linux-musleabihf
cargo build --release --target aarch64-unknown-linux-musl
