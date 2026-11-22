@echo off
rem hla-wgs Start for Windows: double-click. Runs hla-wgs-start.ps1 next to this file.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0hla-wgs-start.ps1" %*
