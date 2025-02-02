#!/usr/bin/env bash

rm -rf .rugix
rm -rf build
mkdir build

export RUGIX_DEV=true

# ./run-bakery bake bundle customized-arm64 --without-compression
# ./run-bakery test test-update-bundle
# ./run-bakery test test-update-image

./run-bakery bundler bundle bundles/script-bundle build/script-bundle.rugixb
./run-bakery test test-update-script