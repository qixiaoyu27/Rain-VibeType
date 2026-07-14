$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
cargo run --manifest-path (Join-Path $Root "src-tauri\Cargo.toml")

