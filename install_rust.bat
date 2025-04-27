@echo off
setlocal enabledelayedexpansion

echo =======================================================
echo Install Rust and Cargo
echo =======================================================

set TEMP_DIR=%TEMP%\rust_install
if not exist "%TEMP_DIR%" mkdir "%TEMP_DIR%"

if defined PROCESSOR_ARCHITEW6432 (
    set ARCH=x64
) else (
    if "%PROCESSOR_ARCHITECTURE%"=="AMD64" (
        set ARCH=x64
    ) else (
        set ARCH=x86
    )
)

echo System architecture: %ARCH%
echo Downloading rustup downloader...

powershell -Command "(New-Object Net.WebClient).DownloadFile('https://win.rustup.rs/!ARCH!', '%TEMP_DIR%\rustup-init.exe')"
if %ERRORLEVEL% neq 0 (
    echo Error while downloading rustup-init.exe
    goto :error
)

echo Download successfully.
echo Installing Rust...

"%TEMP_DIR%\rustup-init.exe" -y --default-toolchain stable --no-modify-path
if %ERRORLEVEL% neq 0 (
    echo Error while installing Rust.
    goto :error
)

set PATH=%USERPROFILE%\.cargo\bin;%PATH%

rustc --version
if %ERRORLEVEL% neq 0 (
    echo Error: Rust not installed correctly.
    goto :error
)

cargo --version
if %ERRORLEVEL% neq 0 (
    echo Error: Cargo not installed correctly.
    goto :error
)

echo =======================================================
echo Install Rust and Cargo successfully!
echo =======================================================
echo.
echo Rust version:
rustc --version
echo.
echo Cargo version:
cargo --version
echo.
echo The path to .cargo\bin has been added to the PATH variable for the current session.
echo To use Rust and Cargo in new command windows, restart your computer or
echo or add the following path to the PATH variable manually:
echo %USERPROFILE%\.cargo\bin
echo.
echo Or enter "Y" for add in PATH envarionment:
set /p ADD_PATH=Add Rust in system PATH? (Y/N): 

if /i "%ADD_PATH%"=="Y" (
    setx PATH "%USERPROFILE%\.cargo\bin;%PATH%" /M
    echo Path added to system PATH variable.
)

goto :eof

:error
echo =======================================================
echo Error while installing Rust и Cargo
echo =======================================================
exit /b 1