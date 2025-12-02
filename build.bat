@echo off
echo ========================================
echo ToneForge Production Build
echo ========================================
echo.

echo [1/2] Building REAPER Extension...
cd reaper-extension\build
cmake --build . --config Release
if %errorlevel% neq 0 (
    echo ERROR: Extension build failed!
    pause
    exit /b 1
)
cd ..\..
echo OK - Extension built
echo.

echo [2/2] Building Tauri App...
cd tauri-app
call npm run tauri build
if %errorlevel% neq 0 (
    echo ERROR: Tauri build failed!
    pause
    exit /b 1
)
cd ..
echo OK - Application built
echo.

echo ========================================
echo Build Complete!
echo ========================================
echo.
echo Output files:
echo - Extension: reaper-extension\build\bin\Release\reaper_toneforge.dll
echo - Application: tauri-app\src-tauri\target\release\toneforge.exe
echo.
echo To install:
echo 1. Copy DLL to %%APPDATA%%\REAPER\UserPlugins\
echo 2. Run toneforge.exe
echo.
pause
