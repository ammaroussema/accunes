param(
    [string]$OutDir = "./android/app/src/main/jniLibs"
)

$ErrorActionPreference = "Stop"

$targets = @(
    @{ Rust = "aarch64-linux-android"; Abi = "arm64-v8a" },
    @{ Rust = "armv7-linux-androideabi"; Abi = "armeabi-v7a" },
    @{ Rust = "x86_64-linux-android"; Abi = "x86_64" }
)

foreach ($t in $targets) {
    rustup target add $t.Rust
}

foreach ($t in $targets) {
    $targetDir = Join-Path $OutDir $t.Abi
    if (-not (Test-Path $targetDir)) {
        New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
    }
    cargo ndk --target $t.Rust --platform 24 build --package accunes_ffi --release
    Copy-Item "target/$($t.Rust)/release/libaccunes_ffi.so" "$targetDir/" -Force
}
