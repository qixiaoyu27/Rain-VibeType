param(
    [string]$Python = "py -3.11",
    [string]$RuntimeVersion = "1.0.0",
    [string]$TorchVersion = "2.11.0",
    [string]$ArtifactBaseUrl = $env:RAIN_RUNTIME_ARTIFACT_BASE_URL,
    [string]$OutputDirectory = ""
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Split-Path -Parent $PSScriptRoot)).Path
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $Root "artifacts\runtimes"
}
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)

function Assert-RepositoryChild([string]$Path) {
    $FullPath = [System.IO.Path]::GetFullPath($Path)
    $RootPrefix = $Root.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $FullPath.StartsWith($RootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to modify a path outside the repository: $FullPath"
    }
    return $FullPath
}

function New-BuildVenv([string]$Path) {
    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath (Assert-RepositoryChild $Path) -Recurse -Force
    }
    $Parts = $Python -split " ", 2
    if ($Parts.Count -eq 2) {
        & $Parts[0] $Parts[1] -m venv $Path
    } else {
        & $Python -m venv $Path
    }
    if ($LASTEXITCODE -ne 0) { throw "Unable to create Python build environment: $Path" }
}

if ([string]::IsNullOrWhiteSpace($ArtifactBaseUrl)) {
    $ArtifactBaseUrl = "https://github.com/qixiaoyu27/Rain-VibeType/releases/latest/download"
}
$ArtifactUri = [Uri]$ArtifactBaseUrl
if ($ArtifactUri.Scheme -ne "https") {
    throw "Runtime artifact base URL must use HTTPS."
}
if ($RuntimeVersion -notmatch '^[A-Za-z0-9._-]+$') {
    throw "RuntimeVersion contains unsupported characters."
}

$OutputDirectory = Assert-RepositoryChild $OutputDirectory
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
Add-Type -AssemblyName System.IO.Compression.FileSystem

$Variants = @(
    [pscustomobject]@{
        Id = "rain-runtime-cpu"
        DisplayName = "CPU inference component"
        Accelerator = "cpu"
        TorchIndex = "https://download.pytorch.org/whl/cpu"
    },
    [pscustomobject]@{
        Id = "rain-runtime-nvidia"
        DisplayName = "NVIDIA GPU inference component"
        Accelerator = "nvidia"
        TorchIndex = "https://download.pytorch.org/whl/cu128"
    }
)

$Components = foreach ($Variant in $Variants) {
    $Venv = Assert-RepositoryChild (Join-Path $Root ".venv-runtime-$($Variant.Accelerator)")
    $VenvPython = Join-Path $Venv "Scripts\python.exe"
    New-BuildVenv $Venv
    & $VenvPython -m pip install --upgrade pip
    if ($LASTEXITCODE -ne 0) { throw "pip upgrade failed for $($Variant.Id)" }
    & $VenvPython -m pip install --index-url $Variant.TorchIndex "torch==$TorchVersion" "torchaudio==$TorchVersion"
    if ($LASTEXITCODE -ne 0) { throw "PyTorch installation failed for $($Variant.Id)" }
    & $VenvPython -m pip install -r (Join-Path $Root "worker\requirements.txt") "pyinstaller>=6.10,<7"
    if ($LASTEXITCODE -ne 0) { throw "Worker dependency installation failed for $($Variant.Id)" }

    $BuildRoot = Assert-RepositoryChild (Join-Path $Root "build\runtimes\$($Variant.Accelerator)")
    $DistRoot = Assert-RepositoryChild (Join-Path $BuildRoot "dist")
    if (Test-Path -LiteralPath $BuildRoot) {
        Remove-Item -LiteralPath $BuildRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $DistRoot | Out-Null
    & $VenvPython -m PyInstaller `
        --noconfirm `
        --clean `
        --onedir `
        --name rain-worker `
        --distpath $DistRoot `
        --workpath (Join-Path $BuildRoot "work") `
        --specpath (Join-Path $BuildRoot "spec") `
        --collect-all funasr `
        --collect-all modelscope `
        --collect-all torch `
        --collect-all torchaudio `
        --collect-all transformers `
        (Join-Path $Root "worker\rain_worker.py")
    if ($LASTEXITCODE -ne 0) { throw "PyInstaller failed for $($Variant.Id)" }

    $WorkerExe = Join-Path $DistRoot "rain-worker\rain-worker.exe"
    if (-not (Test-Path -LiteralPath $WorkerExe)) {
        throw "Worker executable is missing: $WorkerExe"
    }
    $InstalledSize = (Get-ChildItem -LiteralPath (Join-Path $DistRoot "rain-worker") -File -Recurse | Measure-Object -Property Length -Sum).Sum
    $ArchiveName = "$($Variant.Id)-$RuntimeVersion.zip"
    $Archive = Assert-RepositoryChild (Join-Path $OutputDirectory $ArchiveName)
    if (Test-Path -LiteralPath $Archive) { Remove-Item -LiteralPath $Archive -Force }
    [System.IO.Compression.ZipFile]::CreateFromDirectory($DistRoot, $Archive, [System.IO.Compression.CompressionLevel]::Optimal, $false)
    $ArchiveInfo = Get-Item -LiteralPath $Archive
    $Sha256 = (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash.ToLowerInvariant()

    [ordered]@{
        id = $Variant.Id
        display_name = $Variant.DisplayName
        version = $RuntimeVersion
        accelerator = $Variant.Accelerator
        url = "$($ArtifactBaseUrl.TrimEnd('/'))/$ArchiveName"
        archive_size = [long]$ArchiveInfo.Length
        installed_size = [long]$InstalledSize
        sha256 = $Sha256
        executable = "rain-worker/rain-worker.exe"
    }
}

$Manifest = [ordered]@{
    schema_version = 1
    manifest_version = "$RuntimeVersion-$([DateTime]::UtcNow.ToString('yyyyMMddHHmmss'))"
    components = @($Components)
}
$ManifestPath = Assert-RepositoryChild (Join-Path $OutputDirectory "runtime-manifest.json")
$ManifestJson = $Manifest | ConvertTo-Json -Depth 5
[System.IO.File]::WriteAllText($ManifestPath, $ManifestJson, [System.Text.UTF8Encoding]::new($false))

Write-Output "Runtime artifacts created: $OutputDirectory"
Write-Output "Publish both ZIP files and runtime-manifest.json before releasing the base installer."
