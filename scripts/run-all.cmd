@echo off
setlocal

set TOKEN=topsecret
set PORT=7878

echo Starting AuroraOS daemon on 127.0.0.1:%PORT%...
for /f %%i in ('powershell -NoProfile -Command "$p = Start-Process -FilePath cargo -ArgumentList 'run','-p','init','--','--daemon','--listen','127.0.0.1:%PORT%','--auth-token','%TOKEN%' -PassThru; $p.Id"') do set DAEMON_PID=%%i

if not defined DAEMON_PID (
  echo Failed to start daemon.
  exit /b 1
)

timeout /t 3 >nul

echo Starting AuroraOS desktop...
set AURORA_TOKEN=%TOKEN%
cargo run -p desktop-native

echo Stopping daemon...
powershell -NoProfile -Command "Stop-Process -Id %DAEMON_PID% -Force -ErrorAction SilentlyContinue"

endlocal
