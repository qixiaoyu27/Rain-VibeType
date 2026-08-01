param([string]$Python = "python")

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Split-Path -Parent $PSScriptRoot)).Path
$VenvPython = [System.IO.Path]::GetFullPath((Join-Path $Root ".venv\Scripts\python.exe"))

if (-not (Test-Path -LiteralPath $VenvPython)) {
    & $Python -m venv (Join-Path $Root ".venv")
}

& $VenvPython -m pip install --upgrade pip
& $VenvPython -m pip install -r (Join-Path $Root "worker\requirements.txt")
$VenvScripts = Split-Path -Parent $VenvPython
$env:PATH = "$VenvScripts$([System.IO.Path]::PathSeparator)$env:PATH"
Write-Output "Worker ready: $VenvPython"
Write-Output "The virtual environment is active for commands launched from this PowerShell session."
