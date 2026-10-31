#!/usr/bin/env bash
set -euo pipefail

TARGET_ARCHS=("aarch64-linux-android" "armv7-linux-androideabi" "x86_64-linux-android")
OUT_DIR="${1:-./android/app/src/main/jniLibs}"

command -v cargo-ndk >/dev/null 2>&1 || cargo install cargo-ndk

for target in "${TARGET_ARCHS[@]}"; do
    rustup target add "$target"
done

for target in "${TARGET_ARCHS[@]}"; do
    case "$target" in
        "aarch64-linux-android") ABI="arm64-v8a" ;;
        "armv7-linux-androideabi") ABI="armeabi-v7a" ;;
        "x86_64-linux-android") ABI="x86_64" ;;
        *) ABI="$target" ;;
    esac

    mkdir -p "$OUT_DIR/$ABI"
    cargo ndk --target "$target" --platform 24 build --package accunes_ffi --release
    cp "target/$target/release/libaccunes_ffi.so" "$OUT_DIR/$ABI/"
done
