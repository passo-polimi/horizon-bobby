@echo off
setlocal enabledelayedexpansion

echo ============================================================
echo          Horizon Mod Porting Verification Gate (Windows)    
echo ============================================================

set SCRIPT_DIR=%~dp0
cd /d "%SCRIPT_DIR%..\.."

echo [1/4] Checking code formatting (cargo fmt)...
cargo fmt --check --package horizon-mod-api --package distant-horizons --package nvidium --package horizon-mod-harness
if %errorlevel% neq 0 (
    echo [FAILURE] Code formatting check failed! Run 'cargo fmt' to fix.
    exit /b %errorlevel%
)

echo [2/4] Running Clippy with warnings denied...
cargo clippy --all-targets -p horizon-mod-api -p distant-horizons -p nvidium -p horizon-mod-harness -- -D warnings
if %errorlevel% neq 0 (
    echo [FAILURE] Clippy check failed!
    exit /b %errorlevel%
)

echo [3/4] Running API unit tests...
cargo test -p horizon-mod-api --test api_tests
if %errorlevel% neq 0 (
    echo [FAILURE] API unit tests failed!
    exit /b %errorlevel%
)

echo [4/4] Running Java ^<^> Rust differential tests (difftest)...
cargo test -p horizon-mod-harness --test difftest
if %errorlevel% neq 0 (
    echo [FAILURE] Differential tests failed!
    exit /b %errorlevel%
)

echo ============================================================
echo   [SUCCESS] All checks and differential tests passed cleanly!
echo ============================================================

