@echo off
setlocal
set DIRNAME=%~dp0
if "%DIRNAME%" == "" set DIRNAME=.
where gradle >nul 2>nul
if %ERRORLEVEL% equ 0 (
    gradle %*
) else (
    echo Gradle is required. Open in Android Studio or install Gradle on PATH.
    exit /b 1
)
