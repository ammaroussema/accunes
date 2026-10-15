# webport build script!!

Write-Host "Building AccuNES WebAssembly core..." -ForegroundColor Cyan
cargo build --target wasm32-unknown-unknown --release --lib
if ($LASTEXITCODE -ne 0) {
    Write-Host "Cargo build failed!" -ForegroundColor Red
    exit 1
}

Write-Host "Generating web bindings via wasm-bindgen..." -ForegroundColor Cyan
wasm-bindgen --target web --out-name accunes --out-dir web/pkg target/wasm32-unknown-unknown/release/accunes_core.wasm
if ($LASTEXITCODE -ne 0) {
    Write-Host "wasm-bindgen failed!" -ForegroundColor Red
    exit 1
}

Copy-Item -Path accunesicon.ico -Destination web/accunesicon.ico -Force -ErrorAction SilentlyContinue

Write-Host "AccuNES web build complete!" -ForegroundColor Green
Write-Host "to test locally run: python -m http.server -d web 8080" -ForegroundColor Yellow
