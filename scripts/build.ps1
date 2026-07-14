$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
cargo build --release --manifest-path (Join-Path $Root "src-tauri\Cargo.toml")
Write-Output "Built: $Root\src-tauri\target\release\rain-input.exe"

