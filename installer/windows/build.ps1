param(
    [string]$Configuration = "release",
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$appName = "rustext"
$exeSource = Join-Path $repoRoot "target\$Target\$Configuration\$appName.exe"
$packageRoot = Join-Path $repoRoot "target\package\windows\rustext-windows-x64"
$zipPath = Join-Path $repoRoot "target\package\windows\rustext-windows-x64.zip"

if (-not (Test-Path $exeSource)) {
    Write-Host "[1/4] Compilando Rustext para Windows..."
    rustup target add $Target
    cargo build --release --target $Target
}

if (Test-Path $packageRoot) {
    Remove-Item -Recurse -Force $packageRoot
}

New-Item -ItemType Directory -Force -Path $packageRoot | Out-Null

Copy-Item $exeSource -Destination (Join-Path $packageRoot "$appName.exe") -Force
Copy-Item (Join-Path $repoRoot "LICENSE") -Destination $packageRoot -Force
Copy-Item (Join-Path $repoRoot "README.md") -Destination $packageRoot -Force

$installScript = @'
param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Programs\Rustext')
)

$ErrorActionPreference = 'Stop'

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$sourceExe = Join-Path $PSScriptRoot 'rustext.exe'

if (-not (Test-Path $sourceExe)) {
    throw "No se encontró rustext.exe dentro del paquete portable."
}

Copy-Item $sourceExe -Destination (Join-Path $InstallDir 'rustext.exe') -Force

$startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
New-Item -ItemType Directory -Force -Path $startMenu | Out-Null

$shell = New-Object -ComObject WScript.Shell
$shortcutPath = Join-Path $startMenu 'Rustext.lnk'
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = (Join-Path $InstallDir 'rustext.exe')
$shortcut.WorkingDirectory = $InstallDir
$shortcut.IconLocation = (Join-Path $InstallDir 'rustext.exe')
$shortcut.Save()

Write-Host "Rustext instalado en $InstallDir"
'@

Set-Content -Path (Join-Path $packageRoot "install.ps1") -Value $installScript -Encoding UTF8

if (Test-Path $zipPath) {
    Remove-Item -Force $zipPath
}

Compress-Archive -Path (Join-Path $packageRoot '*') -DestinationPath $zipPath -Force

Write-Host "[4/4] Paquete portable creado: $zipPath"
