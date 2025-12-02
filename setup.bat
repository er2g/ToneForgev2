@echo off
echo ========================================
echo ToneForge Setup Script (Windows)
echo ========================================
echo.

echo [1/5] Checking prerequisites...
where cmake >nul 2>&1
if %errorlevel% neq 0 (
    echo ERROR: CMake not found! Please install CMake 3.15+
    pause
    exit /b 1
)

where git >nul 2>&1
if %errorlevel% neq 0 (
    echo ERROR: Git not found! Please install Git
    pause
    exit /b 1
)

where npm >nul 2>&1
if %errorlevel% neq 0 (
    echo ERROR: Node.js/npm not found! Please install Node.js 18+
    pause
    exit /b 1
)

echo OK - All prerequisites found
echo.

echo [2/5] Cloning external dependencies...
mkdir external 2>nul
cd external

if not exist "reaper-sdk" (
    echo Cloning REAPER SDK...
    git clone https://github.com/justinfrankel/reaper-sdk.git
)

if not exist "cpp-httplib" (
    echo Cloning cpp-httplib...
    git clone https://github.com/yhirose/cpp-httplib.git
)

if not exist "json" (
    echo Cloning nlohmann-json...
    git clone https://github.com/nlohmann/json.git
)

cd ..
echo OK - Dependencies ready
echo.

echo [3/5] Building REAPER Extension...
cd reaper-extension
mkdir build 2>nul
cd build

cmake .. -G "Visual Studio 16 2019" -A x64
if %errorlevel% neq 0 (
    echo ERROR: CMake configuration failed!
    echo Make sure you have Visual Studio 2019 or newer installed
    pause
    exit /b 1
)

cmake --build . --config Release
if %errorlevel% neq 0 (
    echo ERROR: Build failed!
    pause
    exit /b 1
)

cd ..\..
echo OK - Extension built
echo.

echo [4/5] Installing REAPER Extension...
set REAPER_PLUGINS=%APPDATA%\REAPER\UserPlugins
if not exist "%REAPER_PLUGINS%" mkdir "%REAPER_PLUGINS%"

copy reaper-extension\build\bin\Release\reaper_toneforge.dll "%REAPER_PLUGINS%\" >nul
if %errorlevel% neq 0 (
    echo WARNING: Could not copy to REAPER plugins folder
    echo You may need to copy manually: reaper-extension\build\bin\Release\reaper_toneforge.dll
    echo To: %REAPER_PLUGINS%
) else (
    echo OK - Extension installed to REAPER
)
echo.

echo [5/5] Setting up Tauri app...
cd tauri-app
call npm install
if %errorlevel% neq 0 (
    echo ERROR: npm install failed!
    pause
    exit /b 1
)
cd ..
echo OK - Tauri app dependencies installed
echo.

echo ========================================
echo Setup Complete!
echo ========================================
echo.
echo Next steps:
echo 1. Start REAPER (make sure Neural DSP or other VST3 plugins are installed)
echo 2. Check Extensions ^> Show Console to verify extension loaded
echo 3. Get a Gemini API key from: https://makersuite.google.com/app/apikey
echo 4. Run: cd tauri-app ^&^& npm run tauri dev
echo.
pause
