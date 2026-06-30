@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0setup-git-hooks.ps1" %*
exit /b %ERRORLEVEL%
