@echo off
rem Development only: start the server and also serve unreviewed DRAFT questions.
rem Double-click this file. Close the window to stop the server.
set GED_SERVE_DRAFTS=1
"%~dp0..\target\release\gedpasser-server.exe"
if errorlevel 1 pause
