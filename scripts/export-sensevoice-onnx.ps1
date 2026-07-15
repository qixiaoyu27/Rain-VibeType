param(
    [Parameter(Mandatory = $true)][string]$ModelPath,
    [string]$OutputDirectory = "",
    [string]$Python = "",
    [switch]$Force
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Split-Path -Parent $PSScriptRoot)).Path
if ([string]::IsNullOrWhiteSpace($Python)) {
    $Python = Join-Path $Root ".venv-worker\Scripts\python.exe"
}
if (-not (Test-Path -LiteralPath $Python -PathType Leaf)) {
    throw "Python Worker environment is missing: $Python"
}
$ModelPath = (Resolve-Path -LiteralPath $ModelPath).Path
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = $ModelPath
}
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)

& $Python -m pip install -r (Join-Path $Root "worker\export-requirements.txt")
if ($LASTEXITCODE -ne 0) { throw "Failed to install ONNX export dependency." }
$ExportArguments = @((Join-Path $Root "worker\export_sensevoice_onnx.py"), $ModelPath, "--output", $OutputDirectory)
if ($Force) { $ExportArguments += "--force" }
& $Python @ExportArguments
if ($LASTEXITCODE -ne 0) { throw "SenseVoice ONNX export failed." }

Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $OutputDirectory "model.onnx"), (Join-Path $OutputDirectory "tokens.txt")
