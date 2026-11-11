# Build the single release binary: target\release\gedpasser-server.exe
# 1. regenerate TypeScript API types from Rust (ts-rs, via cargo test)
# 2. build the React app into web\dist
# 3. build the server, which embeds web\dist
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)

cargo test -q
if ($LASTEXITCODE -ne 0) { throw 'type generation (cargo test) failed' }

npm --prefix web ci
if ($LASTEXITCODE -ne 0) { throw 'npm ci failed' }
npm --prefix web run build
if ($LASTEXITCODE -ne 0) { throw 'web build failed' }

cargo build --release -p gedpasser-server
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

Write-Host 'Built target\release\gedpasser-server.exe (run it from the repo root so it finds content\)'
