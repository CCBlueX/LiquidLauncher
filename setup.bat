@echo off
setlocal EnableExtensions EnableDelayedExpansion

rem ===========================================================================
rem  LiquidLauncher - Windows development environment setup
rem
rem  Detects every build prerequisite and installs ONLY what is missing.
rem  Nothing is reinstalled or upgraded if it is already present.
rem
rem  Usage:
rem    setup.bat            Check, then install anything missing
rem    setup.bat /check     Check only, install nothing (exit 1 if incomplete)
rem ===========================================================================

cd /d "%~dp0"

rem --- Hoist paths containing parentheses out of any IF/FOR block ------------
set "PF86=%ProgramFiles(x86)%"
set "PF64=%ProgramFiles%"
set "VSWHERE=%PF86%\Microsoft Visual Studio\Installer\vswhere.exe"
set "TMPDIR=%TEMP%\liquidlauncher-setup"
set "WV2GUID={F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
set "SELF=%~f0"

rem --- Architecture-specific bits -------------------------------------------
set "RUSTUP_URL=https://win.rustup.rs/x86_64"
set "VCCOMP=Microsoft.VisualStudio.Component.VC.Tools.x86.x64"
if /i "%PROCESSOR_ARCHITECTURE%"=="ARM64" (
    set "RUSTUP_URL=https://win.rustup.rs/aarch64"
    set "VCCOMP=Microsoft.VisualStudio.Component.VC.Tools.ARM64"
)

rem --- Mode -----------------------------------------------------------------
set "MODE=full"
if /i "%~1"=="/check"        set "MODE=check"
if /i "%~1"=="--check"       set "MODE=check"
if /i "%~1"=="-c"            set "MODE=check"
if /i "%~1"=="--admin-phase" set "MODE=admin"

echo.
echo  ===========================================================
echo   LiquidLauncher - development environment setup
echo  ===========================================================
echo.

call :detect
call :report

if "%MODE%"=="check"  goto :finish_check
if "%MODE%"=="admin"  goto :admin_phase_only

if "%MISSING%"=="0" (
    echo  Everything is already installed - nothing to download.
    echo.
    goto :project_deps
)

rem --- Work out whether we need elevation ------------------------------------
set "NEED_ADMIN=0"
if "%NEED_VS%"=="1"   set "NEED_ADMIN=1"
if "%NEED_NODE%"=="1" set "NEED_ADMIN=1"
if "%NEED_WV2%"=="1"  set "NEED_ADMIN=1"

if not exist "%TMPDIR%" mkdir "%TMPDIR%" >nul 2>&1

if "%NEED_ADMIN%"=="1" (
    call :is_admin
    if "!IS_ADMIN!"=="1" (
        call :install_admin_items
    ) else (
        echo  Some components need administrator rights.
        echo  A UAC prompt will appear - approve it to continue.
        echo.
        powershell -NoProfile -ExecutionPolicy Bypass -Command "try { Start-Process -FilePath '!SELF!' -ArgumentList '--admin-phase' -Verb RunAs -Wait -ErrorAction Stop } catch { exit 1 }"
        if errorlevel 1 (
            echo.
            echo  [X] Elevation was cancelled or failed.
            echo      Re-run this script as administrator to finish.
            echo.
            goto :finish_fail
        )
    )
)

call :install_user_items

rem --- Re-check now that everything has been installed ------------------------
echo.
echo  Re-checking prerequisites...
echo.
call :detect
call :report

if not "%MISSING%"=="0" (
    echo  [X] Some prerequisites are still missing - see above.
    echo.
    goto :finish_fail
)

:project_deps
echo  Installing project dependencies ^(bun install^)...
echo.
call bun install
if errorlevel 1 (
    echo.
    echo  [X] "bun install" failed.
    echo.
    goto :finish_fail
)

echo.
echo  ===========================================================
echo   Setup complete.
echo.
echo   Run the launcher in dev mode:   bun run tauri dev
echo   Build a release binary:         bun run tauri build
echo.
echo   NOTE: if you installed Rust or bun just now, open a NEW
echo         terminal first so the updated PATH is picked up.
echo  ===========================================================
echo.
goto :finish_ok


rem ===========================================================================
rem  DETECTION - sets NEED_* flags and *_VER strings. Installs nothing.
rem ===========================================================================
:detect
set "NEED_VS=0"
set "NEED_WV2=0"
set "NEED_RUST=0"
set "NEED_NIGHTLY=0"
set "NEED_BUN=0"
set "NEED_NODE=0"
set "MISSING=0"

set "VSPATH="
set "WV2VER="
set "RUSTVER="
set "BUNVER="
set "NODEVER="

rem Pick up tools installed earlier in this same session / by this script,
rem whose PATH change has not reached this console yet.
if exist "%USERPROFILE%\.cargo\bin\rustup.exe" set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
if exist "%USERPROFILE%\.bun\bin\bun.exe"      set "PATH=%USERPROFILE%\.bun\bin;%PATH%"
if exist "%PF64%\nodejs\node.exe"              set "PATH=%PF64%\nodejs;%PATH%"

rem --- MSVC C++ build tools (the Rust linker on Windows) ---------------------
rem NOTE: !VSWHERE! (delayed) not %VSWHERE% - the path contains "(x86)" and the
rem       parentheses in the value would close this block during parsing.
if exist "%VSWHERE%" (
    for /f "usebackq delims=" %%I in (`"!VSWHERE!" -products * -latest -requires !VCCOMP! -property installationPath 2^>nul`) do set "VSPATH=%%I"
)
if not defined VSPATH set "NEED_VS=1"

rem --- WebView2 runtime (bundled with Win11, may be absent on Win10) ---------
for %%K in (
    "HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\%WV2GUID%"
    "HKLM\SOFTWARE\Microsoft\EdgeUpdate\Clients\%WV2GUID%"
    "HKCU\SOFTWARE\Microsoft\EdgeUpdate\Clients\%WV2GUID%"
) do (
    if not defined WV2VER (
        for /f "tokens=3" %%V in ('reg query %%K /v pv 2^>nul ^| find "pv"') do set "WV2VER=%%V"
    )
)
if not defined WV2VER      set "NEED_WV2=1"
if "%WV2VER%"=="0.0.0.0"   set "NEED_WV2=1"

rem --- Rust toolchain --------------------------------------------------------
where rustup >nul 2>&1
if errorlevel 1 set "NEED_RUST=1"
where cargo  >nul 2>&1
if errorlevel 1 set "NEED_RUST=1"

if "%NEED_RUST%"=="0" (
    for /f "tokens=2" %%V in ('rustc --version 2^>nul') do set "RUSTVER=%%V"
    rem rust-toolchain.toml pins the nightly channel
    rustup toolchain list 2>nul | find /i "nightly" >nul
    if errorlevel 1 set "NEED_NIGHTLY=1"
)

rem --- bun -------------------------------------------------------------------
where bun >nul 2>&1
if errorlevel 1 (
    set "NEED_BUN=1"
) else (
    for /f "delims=" %%V in ('bun --version 2^>nul') do set "BUNVER=%%V"
)

rem --- Node.js ---------------------------------------------------------------
where node >nul 2>&1
if errorlevel 1 (
    set "NEED_NODE=1"
) else (
    for /f "delims=" %%V in ('node --version 2^>nul') do set "NODEVER=%%V"
)

for %%F in (NEED_VS NEED_WV2 NEED_RUST NEED_NIGHTLY NEED_BUN NEED_NODE) do (
    if "!%%F!"=="1" set /a MISSING+=1
)
goto :eof


rem ===========================================================================
rem  REPORT
rem ===========================================================================
:report
call :line "Visual Studio C++ build tools" "%NEED_VS%"  "%VSPATH%"
call :line "WebView2 runtime"              "%NEED_WV2%" "%WV2VER%"
call :line "Rust (rustup + cargo)"         "%NEED_RUST%" "%RUSTVER%"
if "%NEED_RUST%"=="0" call :line "Rust nightly toolchain" "%NEED_NIGHTLY%" "installed"
call :line "bun"                           "%NEED_BUN%"  "%BUNVER%"
call :line "Node.js"                       "%NEED_NODE%" "%NODEVER%"
echo.
goto :eof

:line
set "LBL=%~1                                   "
set "LBL=!LBL:~0,34!"
rem VSPATH can contain "(x86)" - stash it before entering the block below
set "VAL=%~3"
if "%~2"=="1" (
    echo   [ MISSING ]  !LBL!
) else (
    echo   [   OK    ]  !LBL!!VAL!
)
goto :eof


rem ===========================================================================
rem  ELEVATED CHILD - installs only the machine-wide components, then exits
rem ===========================================================================
:admin_phase_only
if not exist "%TMPDIR%" mkdir "%TMPDIR%" >nul 2>&1
if "%MISSING%"=="0" goto :finish_ok
call :install_admin_items
echo.
echo  Administrator phase finished - this window will close shortly.
timeout /t 8 >nul
goto :finish_ok


rem ===========================================================================
rem  INSTALLERS - machine-wide (require administrator)
rem ===========================================================================
:install_admin_items
if "%NEED_VS%"=="1"   call :install_vs
if "%NEED_WV2%"=="1"  call :install_webview2
if "%NEED_NODE%"=="1" call :install_node
goto :eof

:install_vs
echo.
echo  --^> Installing Visual Studio Build Tools ^(C++ workload^)
echo      This is the largest download ^(~3-4 GB^) and may take a while.
call :download "https://aka.ms/vs/17/release/vs_BuildTools.exe" "%TMPDIR%\vs_BuildTools.exe"
if not exist "%TMPDIR%\vs_BuildTools.exe" (
    echo      [X] Download failed. Install manually: https://aka.ms/vs/17/release/vs_BuildTools.exe
    goto :eof
)
"%TMPDIR%\vs_BuildTools.exe" --passive --wait --norestart --nocache --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended
set "RC=%ERRORLEVEL%"
if "%RC%"=="3010" echo      Installed - a reboot is required before building.
if not "%RC%"=="0" if not "%RC%"=="3010" echo      [X] Build Tools installer exited with code %RC%.
goto :eof

:install_webview2
echo.
echo  --^> Installing WebView2 runtime
call :download "https://go.microsoft.com/fwlink/p/?LinkId=2124703" "%TMPDIR%\MicrosoftEdgeWebview2Setup.exe"
if not exist "%TMPDIR%\MicrosoftEdgeWebview2Setup.exe" (
    echo      [X] Download failed. Install manually: https://developer.microsoft.com/microsoft-edge/webview2/
    goto :eof
)
"%TMPDIR%\MicrosoftEdgeWebview2Setup.exe" /silent /install
goto :eof

:install_node
echo.
echo  --^> Installing Node.js LTS
where winget >nul 2>&1
if errorlevel 1 (
    echo      [X] winget not available. Install Node manually: https://nodejs.org/
    goto :eof
)
winget install --id OpenJS.NodeJS.LTS -e --silent --accept-source-agreements --accept-package-agreements
set "PATH=%PF64%\nodejs;%PATH%"
goto :eof


rem ===========================================================================
rem  INSTALLERS - per-user (no administrator needed)
rem ===========================================================================
:install_user_items
if "%NEED_RUST%"=="1"    call :install_rust
if "%NEED_NIGHTLY%"=="1" call :install_nightly
if "%NEED_BUN%"=="1"     call :install_bun
goto :eof

:install_rust
echo.
echo  --^> Installing Rust ^(rustup, nightly channel^)
call :download "%RUSTUP_URL%" "%TMPDIR%\rustup-init.exe"
if not exist "%TMPDIR%\rustup-init.exe" (
    echo      [X] Download failed. Install manually: https://rustup.rs/
    goto :eof
)
"%TMPDIR%\rustup-init.exe" -y --default-toolchain nightly --profile default
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
goto :eof

:install_nightly
echo.
echo  --^> Adding the Rust nightly toolchain ^(pinned by rust-toolchain.toml^)
call rustup toolchain install nightly
goto :eof

:install_bun
echo.
echo  --^> Installing bun
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm bun.sh/install.ps1 | iex"
set "PATH=%USERPROFILE%\.bun\bin;%PATH%"
where bun >nul 2>&1
if errorlevel 1 (
    echo      PowerShell installer did not succeed, trying npm...
    where npm >nul 2>&1
    if not errorlevel 1 call npm install -g bun
)
goto :eof


rem ===========================================================================
rem  HELPERS
rem ===========================================================================
:is_admin
set "IS_ADMIN=0"
net session >nul 2>&1
if not errorlevel 1 set "IS_ADMIN=1"
goto :eof

:download
rem %1 = url   %2 = destination file
if exist "%~2" del /q "%~2" >nul 2>&1
where curl >nul 2>&1
if not errorlevel 1 (
    curl -L --fail --silent --show-error -o "%~2" "%~1"
    if exist "%~2" goto :eof
)
powershell -NoProfile -ExecutionPolicy Bypass -Command "$ProgressPreference='SilentlyContinue'; try { Invoke-WebRequest -Uri '%~1' -OutFile '%~2' -UseBasicParsing } catch { exit 1 }"
goto :eof


rem ===========================================================================
rem  EXITS
rem ===========================================================================
:finish_check
if "%MISSING%"=="0" (
    echo  All prerequisites are present.
    echo.
    goto :finish_ok
)
echo  %MISSING% prerequisite^(s^) missing. Run setup.bat to install them.
echo.
goto :finish_fail

:finish_ok
endlocal
exit /b 0

:finish_fail
endlocal
exit /b 1
