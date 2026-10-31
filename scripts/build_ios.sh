#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${1:-./ios/Frameworks}"

rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios

cargo build --package accunes_ffi --target aarch64-apple-ios --release
cargo build --package accunes_ffi --target aarch64-apple-ios-sim --release
cargo build --package accunes_ffi --target x86_64-apple-ios --release

mkdir -p target/ios-sim-lipo
lipo -create \
    target/aarch64-apple-ios-sim/release/libaccunes_ffi.a \
    target/x86_64-apple-ios/release/libaccunes_ffi.a \
    -output target/ios-sim-lipo/libaccunes_ffi.a

rm -rf "$OUT_DIR/AccuNES.xcframework"
mkdir -p "$OUT_DIR"

xcodebuild -create-xcframework \
    -library target/aarch64-apple-ios/release/libaccunes_ffi.a \
    -headers crates/accunes_ffi/include \
    -library target/ios-sim-lipo/libaccunes_ffi.a \
    -headers crates/accunes_ffi/include \
    -output "$OUT_DIR/AccuNES.xcframework"
