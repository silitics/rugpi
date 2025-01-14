#!/bin/bash

CARGO_TARGET_DIR=$(pwd)/target/lima;
export CARGO_TARGET_DIR;
export PATH=$CARGO_TARGET_DIR/debug:$PATH;
export RUST_BACKTRACE=1;
