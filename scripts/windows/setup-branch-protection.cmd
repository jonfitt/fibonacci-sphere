@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0setup-branch-protection.ps1" %*
exit /b %ERRORLEVEL%
