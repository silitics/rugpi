#!/usr/bin/env bash

set -euo pipefail

export CC_x86_64_unknown_linux_musl=x86_64-linux-gnu-gcc
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-Clink-self-contained=yes -Clinker=rust-lld"

export CC_aarch64_unknown_linux_musl=aarch64-linux-gnu-gcc
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-Clink-self-contained=yes -Clinker=rust-lld"

export CC_arm_unknown_linux_musleabihf=arm-linux-gnueabihf-gcc
export CARGO_TARGET_ARM_UNKNOWN_LINUX_MUSLEABIHF_RUSTFLAGS="-Clink-self-contained=yes -Clinker=rust-lld"

exec "$@"
