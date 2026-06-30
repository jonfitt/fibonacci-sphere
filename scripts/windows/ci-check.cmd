@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0ci-check.ps1" %*
exit /b %ERRORLEVEL%
