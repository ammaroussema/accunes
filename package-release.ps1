param([string]$Profile = "release")

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location -LiteralPath $ProjectRoot

if ($PSVersionTable.PSEdition -eq "Core") {
    $HostOS = if ($IsWindows) { "windows" } elseif ($IsLinux) { "linux" } elseif ($IsMacOS) { "macos" } else { "unknown" }
} else {
    $HostOS = "windows"
}
$HostArch = if ($HostOS -eq "windows") {
    $arch = $env:PROCESSOR_ARCHITECTURE
    if ($arch -eq "ARM64") { "arm64" } elseif ($arch -match "86") { "x32" } else { "x64" }
} else {
    switch ((& uname -m).Trim().ToLower()) {
        "x86_64"  { "x64" }
        "aarch64" { "arm64" }
        "arm64"   { "arm64" }
        "i686"    { "x32" }
        default   { "x64" }
    }
}

switch ($Profile) {
    "release" {
        $BuildArgs = @("build", "--release")
        $TargetDir = "target/release"
        $Suffix = switch ($HostOS) {
            "windows" { "win64" }
            "linux"   { "linux-$HostArch" }
            "macos"   { "macos-$HostArch" }
            default   { "unknown-$HostArch" }
        }
    }
    "debug" {
        $BuildArgs = @("build")
        $TargetDir = "target/debug"
        $Suffix = switch ($HostOS) {
            "windows" { "debug" }
            "linux"   { "linux-$HostArch-debug" }
            "macos"   { "macos-$HostArch-debug" }
            default   { "unknown-$HostArch-debug" }
        }
    }
    "x32" {
        $BuildArgs = @("build", "--release", "--target", "i686-pc-windows-msvc")
        $TargetDir = "target/i686-pc-windows-msvc/release"
        $Suffix = "win32"
    }
    "x32debug" {
        $BuildArgs = @("build", "--target", "i686-pc-windows-msvc")
        $TargetDir = "target/i686-pc-windows-msvc/debug"
        $Suffix = "win32-debug"
    }
    "arm64" {
        $BuildArgs = @("build", "--release", "--target", "aarch64-pc-windows-msvc")
        $TargetDir = "target/aarch64-pc-windows-msvc/release"
        $Suffix = "winarm64"
    }
    "arm64debug" {
        $BuildArgs = @("build", "--target", "aarch64-pc-windows-msvc")
        $TargetDir = "target/aarch64-pc-windows-msvc/debug"
        $Suffix = "winarm64-debug"
    }
    "linux64" {
        $BuildArgs = @("build", "--release", "--target", "x86_64-unknown-linux-gnu")
        $TargetDir = "target/x86_64-unknown-linux-gnu/release"
        $Suffix = "linux-x64"
    }
    "linuxarm64" {
        $BuildArgs = @("build", "--release", "--target", "aarch64-unknown-linux-gnu")
        $TargetDir = "target/aarch64-unknown-linux-gnu/release"
        $Suffix = "linux-arm64"
    }
    "macosx64" {
        $BuildArgs = @("build", "--release", "--target", "x86_64-apple-darwin")
        $TargetDir = "target/x86_64-apple-darwin/release"
        $Suffix = "macos-x64"
    }
    "macosarm64" {
        $BuildArgs = @("build", "--release", "--target", "aarch64-apple-darwin")
        $TargetDir = "target/aarch64-apple-darwin/release"
        $Suffix = "macos-arm64"
    }
    default {
        $BuildArgs = @("build", "--profile", $Profile)
        $TargetDir = "target/$Profile"
        $Suffix = $Profile
    }
}

Write-Host "Building $Profile ($HostOS/$HostArch)..."
& "cargo" $BuildArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$version = [regex]::Match((Get-Content "Cargo.toml" -Raw), 'version = "(.+)"').Groups[1].Value

$PackageDir = Join-Path $ProjectRoot "$TargetDir/accunes"
$TargetIndex = [array]::IndexOf($BuildArgs, "--target")
if ($TargetIndex -ge 0) {
    $TargetTriple = $BuildArgs[$TargetIndex + 1]
    $TargetOS = if ($TargetTriple -like "*windows*") { "windows" } elseif ($TargetTriple -like "*apple*") { "macos" } else { "linux" }
} else {
    $TargetOS = $HostOS
}
$ExeExt = if ($TargetOS -eq "windows") { ".exe" } else { "" }
$ExeSource = Join-Path $ProjectRoot "$TargetDir/accunes$ExeExt"
$ExeDest = Join-Path $PackageDir "accunes$ExeExt"

if (-not (Test-Path -LiteralPath $PackageDir)) {
    New-Item -ItemType Directory -Path $PackageDir -Force | Out-Null
}

if (Test-Path -LiteralPath $ExeSource) {
    Copy-Item -LiteralPath $ExeSource -Destination $ExeDest -Force
    Write-Host "Copied accunes$ExeExt -> $PackageDir"
}

$DataSource = Join-Path $ProjectRoot "data"
if (Test-Path -LiteralPath $DataSource -PathType Container) {
    Copy-Item -LiteralPath $DataSource -Destination $PackageDir -Recurse -Force
    Write-Host "Copied data/ -> $PackageDir"
}

$ZipName = "accunes-$version-$Suffix.zip"
$ZipPath = Join-Path $ProjectRoot "$TargetDir/$ZipName"
if (Test-Path -LiteralPath $ZipPath) { Remove-Item -LiteralPath $ZipPath -Force }

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::Open($ZipPath, [System.IO.Compression.ZipArchiveMode]::Create)
$packageDirFull = Resolve-Path -LiteralPath $PackageDir
Get-ChildItem -LiteralPath $packageDirFull -Recurse -File | ForEach-Object {
    $relativePath = "accunes/$($_.FullName.Substring($packageDirFull.Path.Length + 1).Replace('\', '/'))"
    $entry = $zip.CreateEntry($relativePath, [System.IO.Compression.CompressionLevel]::Optimal)
    $stream = $entry.Open()
    $fileBytes = [System.IO.File]::ReadAllBytes($_.FullName)
    $stream.Write($fileBytes, 0, $fileBytes.Length)
    $stream.Dispose()
}
$zip.Dispose()
Write-Host "Created: $ZipPath"