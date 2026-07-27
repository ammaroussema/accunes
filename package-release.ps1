param([string]$Profile = "release")

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location -LiteralPath $ProjectRoot

switch ($Profile) {
    "release" {
        $BuildArgs = @("build", "--release")
        $TargetDir = "target\release"
        $Suffix = "win64"
    }
    "debug" {
        $BuildArgs = @("build")
        $TargetDir = "target\debug"
        $Suffix = "debug"
    }
    "x32" {
        $BuildArgs = @("build", "--release", "--target", "i686-pc-windows-msvc")
        $TargetDir = "target\i686-pc-windows-msvc\release"
        $Suffix = "win32"
    }
    "x32debug" {
        $BuildArgs = @("build", "--target", "i686-pc-windows-msvc")
        $TargetDir = "target\i686-pc-windows-msvc\debug"
        $Suffix = "win32-debug"
    }
    "arm64" {
        $BuildArgs = @("build", "--release", "--target", "aarch64-pc-windows-msvc")
        $TargetDir = "target\aarch64-pc-windows-msvc\release"
        $Suffix = "winarm64"
    }
    "arm64debug" {
        $BuildArgs = @("build", "--target", "aarch64-pc-windows-msvc")
        $TargetDir = "target\aarch64-pc-windows-msvc\debug"
        $Suffix = "winarm64-debug"
    }
    default {
        $BuildArgs = @("build", "--profile", $Profile)
        $TargetDir = "target\$Profile"
        $Suffix = $Profile
    }
}

Write-Host "Building $Profile..."
& "cargo" $BuildArgs
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$version = [regex]::Match((Get-Content "Cargo.toml" -Raw), 'version = "(.+)"').Groups[1].Value

$PackageDir = Join-Path $ProjectRoot "$TargetDir\accunes"
$ExeSource = Join-Path $ProjectRoot "$TargetDir\accunes.exe"
$ExeDest = Join-Path $PackageDir "accunes.exe"

if (-not (Test-Path -LiteralPath $PackageDir)) {
    New-Item -ItemType Directory -Path $PackageDir -Force | Out-Null
}

if (Test-Path -LiteralPath $ExeSource) {
    Copy-Item -LiteralPath $ExeSource -Destination $ExeDest -Force
    Write-Host "Copied accunes.exe -> $PackageDir"
}

$ZipName = "accunes-$version-$Suffix.zip"
$ZipPath = Join-Path $ProjectRoot "$TargetDir\$ZipName"
if (Test-Path -LiteralPath $ZipPath) { Remove-Item -LiteralPath $ZipPath -Force }

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::Open($ZipPath, [System.IO.Compression.ZipArchiveMode]::Create)
$packageDirFull = Resolve-Path -LiteralPath $PackageDir
$dirEntry = $zip.CreateEntry("accunes/")
$dirEntry.Dispose()
Get-ChildItem -LiteralPath $packageDirFull -Recurse | ForEach-Object {
    $relativePath = "accunes/$($_.FullName.Substring($packageDirFull.Path.Length + 1).Replace('\', '/'))"
    if ($_.PSIsContainer) {
        $entry = $zip.CreateEntry("$relativePath/")
        $entry.Dispose()
    } else {
        $entry = $zip.CreateEntry($relativePath, [System.IO.Compression.CompressionLevel]::Optimal)
        $stream = $entry.Open()
        $fileBytes = [System.IO.File]::ReadAllBytes($_.FullName)
        $stream.Write($fileBytes, 0, $fileBytes.Length)
        $stream.Dispose()
    }
}
$zip.Dispose()
Write-Host "Created: $ZipPath"
