@echo off
:: Set up MSVC environment
echo Setting up MSVC environment...
set "VCVARSALL_PATH=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvarsall.bat"
if not "%VCVARSALL_PATH%"=="NOT_FOUND" (
    call "%VCVARSALL_PATH%" x64
    echo MSVC environment set up successfully
) else (
    echo WARNING: Could not find vcvarsall.bat, MSVC environment variables may not be set up
)

:: Add netter to PATH
echo Adding E:\projects\rust\cli\target\release to PATH...
setx PATH "%PATH%;E:\projects\rust\cli\target\release"

