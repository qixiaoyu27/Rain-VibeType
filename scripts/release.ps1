param(
    [string]$Python = "py -3.11",
    [string]$RuntimeVersion = "1.0.0",
    [string]$TorchVersion = "2.11.0",
    [string]$ArtifactBaseUrl = $env:RAIN_RUNTIME_ARTIFACT_BASE_URL,
    [switch]$SkipRuntimes
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Split-Path -Parent $PSScriptRoot)).Path
$ReleaseBase = "https://github.com/qixiaoyu27/Rain-VibeType/releases/latest/download"
$EndpointDefaults = @{
    RAIN_UPDATE_ENDPOINT = "$ReleaseBase/latest.json"
    RAIN_MODEL_MANIFEST_ENDPOINT = "$ReleaseBase/models.json"
    RAIN_RUNTIME_MANIFEST_ENDPOINT = "$ReleaseBase/runtime-manifest.json"
}
foreach ($Name in $EndpointDefaults.Keys) {
    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($Name))) {
        [Environment]::SetEnvironmentVariable($Name, $EndpointDefaults[$Name], "Process")
    }
}
if ([string]::IsNullOrWhiteSpace($ArtifactBaseUrl)) {
    $ArtifactBaseUrl = $ReleaseBase
}

foreach ($Name in "RAIN_UPDATE_PUBLIC_KEY", "TAURI_SIGNING_PRIVATE_KEY") {
    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($Name))) {
        throw "Release signing variable $Name is required."
    }
}

$RuntimeArtifacts = Join-Path $Root "artifacts\runtimes"
New-Item -ItemType Directory -Force -Path $RuntimeArtifacts | Out-Null
Copy-Item -LiteralPath (Join-Path $Root "src-tauri\resources\models.json") -Destination (Join-Path $RuntimeArtifacts "models.json") -Force

if (-not $SkipRuntimes) {
    & (Join-Path $PSScriptRoot "build-runtimes.ps1") `
        -Python $Python `
        -RuntimeVersion $RuntimeVersion `
        -TorchVersion $TorchVersion `
        -ArtifactBaseUrl $ArtifactBaseUrl
    if ($LASTEXITCODE -ne 0) { throw "Runtime component build failed." }
}

Push-Location $Root
try {
    npm install
    npm run build
} finally {
    Pop-Location
}

Write-Output "Lightweight NSIS bundle and signed updater artifacts created under src-tauri\target\release\bundle\nsis"
