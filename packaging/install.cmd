@echo off
rem miniVE installer — Windows CMD.
rem
rem   curl -fsSL https://sahilsidhu7.github.io/minive-landing/install.cmd -o install.cmd && install.cmd && del install.cmd
rem
rem Thin bootstrap: fetches install.ps1 and runs it with PowerShell.
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://sahilsidhu7.github.io/minive-landing/install.ps1 | iex"
