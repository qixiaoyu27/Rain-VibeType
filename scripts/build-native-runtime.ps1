param(
    [string]$RuntimeVersion = "0.1.0",
    [string]$ArtifactBaseUrl = "https://github.com/qixiaoyu27/Rain-VibeType/releases/latest/download",
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
    $Prefix = $Root.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $FullPath.StartsWith($Prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to modify a path outside the repository: $FullPath"
    }
    return $FullPath
}

$ArtifactUri = [Uri]$ArtifactBaseUrl
if ($ArtifactUri.Scheme -ne "https") { throw "ArtifactBaseUrl must use HTTPS." }
if ($RuntimeVersion -notmatch '^[A-Za-z0-9._-]+$') { throw "RuntimeVersion contains unsupported characters." }
$OutputDirectory = Assert-RepositoryChild $OutputDirectory
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null

& cargo build --release --manifest-path (Join-Path $Root "native-worker\Cargo.toml")
if ($LASTEXITCODE -ne 0) { throw "Native Worker build failed." }

$ExecutableName = if ($IsWindows -or $env:OS -eq "Windows_NT") { "rain-native-worker.exe" } else { "rain-native-worker" }
$BuiltWorker = Join-Path $Root "native-worker\target\release\$ExecutableName"
if (-not (Test-Path -LiteralPath $BuiltWorker -PathType Leaf)) { throw "Native Worker executable is missing: $BuiltWorker" }

$BuildRoot = Assert-RepositoryChild (Join-Path $Root "build\native-runtime")
if (Test-Path -LiteralPath $BuildRoot) { Remove-Item -LiteralPath $BuildRoot -Recurse -Force }
$WorkerDirectory = Join-Path $BuildRoot "dist\rain-worker"
New-Item -ItemType Directory -Force -Path $WorkerDirectory | Out-Null
$PackagedName = if ($ExecutableName.EndsWith('.exe')) { "rain-worker.exe" } else { "rain-worker" }
$PackagedWorker = Join-Path $WorkerDirectory $PackagedName
Copy-Item -LiteralPath $BuiltWorker -Destination $PackagedWorker

Add-Type -AssemblyName System.IO.Compression.FileSystem
$ArchiveName = "rain-runtime-onnx-cpu-$RuntimeVersion.zip"
$Archive = Assert-RepositoryChild (Join-Path $OutputDirectory $ArchiveName)
if (Test-Path -LiteralPath $Archive) { Remove-Item -LiteralPath $Archive -Force }
[System.IO.Compression.ZipFile]::CreateFromDirectory((Join-Path $BuildRoot "dist"), $Archive, [System.IO.Compression.CompressionLevel]::Optimal, $false)
$ArchiveInfo = Get-Item -LiteralPath $Archive
$Component = [ordered]@{
    id = "rain-runtime-onnx-cpu"
    display_name = "SenseVoice native CPU inference component"
    version = $RuntimeVersion
    accelerator = "cpu"
    adapter_types = @("sensevoice")
    url = "$($ArtifactBaseUrl.TrimEnd('/'))/$ArchiveName"
    archive_size = [long]$ArchiveInfo.Length
    installed_size = [long](Get-Item -LiteralPath $PackagedWorker).Length
    sha256 = (Get-FileHash -LiteralPath $Archive -Algorithm SHA256).Hash.ToLowerInvariant()
    executable = if ($ExecutableName.EndsWith('.exe')) { "rain-worker/rain-worker.exe" } else { "rain-worker/rain-worker" }
}
$ComponentPath = Assert-RepositoryChild (Join-Path $OutputDirectory "runtime-component-onnx-cpu.json")
[System.IO.File]::WriteAllText($ComponentPath, ($Component | ConvertTo-Json -Depth 4), [System.Text.UTF8Encoding]::new($false))
Write-Output "Native runtime artifact: $Archive"
Write-Output "Merge $ComponentPath into runtime-manifest.json only after the ONNX parity test passes."
