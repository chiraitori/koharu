@echo off
title Koharu Dev
cd /d "%~dp0"
set "PATH=C:\Users\Asus\.cargo\bin;C:\Users\Asus\.bun\bin;%PATH%"
set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
echo ========================================================
echo Starting Koharu Desktop Application with MCP Support...
echo ========================================================
bun dev
