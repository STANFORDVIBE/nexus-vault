# Release build: signed installer + latest.json for in-app updates.
#
#   powershell -ExecutionPolicy Bypass -File scripts/build-release.ps1
#
# The private signing key is read from $env:NEXUS_VAULT_SIGNING_KEY or
# %USERPROFILE%\.tauri\nexus-vault.key. It never goes into the repository.
# Upload the installer, its .sig and latest.json to a GitHub release tagged v<version>.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$keyPath = $env:NEXUS_VAULT_SIGNING_KEY
if (-not $keyPath) { $keyPath = Join-Path $env:USERPROFILE '.tauri\nexus-vault.key' }
if (-not (Test-Path $keyPath)) { throw "Signing key not found: $keyPath" }

$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw $keyPath
if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) { $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = '' }
try {
    npx tauri build --config src-tauri/tauri.release.conf.json
    if ($LASTEXITCODE -ne 0) { throw 'tauri build failed' }
} finally {
    Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
}

$version = (Get-Content -Raw src-tauri/tauri.conf.json | ConvertFrom-Json).version
$nsis = 'src-tauri/target/release/bundle/nsis'
$installer = Get-ChildItem $nsis -Filter "*_${version}_x64-setup.exe" | Select-Object -First 1
if (-not $installer) { throw "Installer for $version not found in $nsis" }
$signature = (Get-Content -Raw "$($installer.FullName).sig").Trim()

# GitHub replaces spaces in asset names with dots
$asset = $installer.Name -replace ' ', '.'
$latest = [ordered]@{
    version   = $version
    notes     = "Nexus Vault $version"
    pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    platforms = [ordered]@{
        'windows-x86_64' = [ordered]@{
            signature = $signature
            url       = "https://github.com/STANFORDVIBE/nexus-vault/releases/download/v$version/$asset"
        }
    }
}
$json = $latest | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText((Join-Path $root "$nsis/latest.json"), $json)

Write-Host ''
Write-Host "Ready for release v${version}:"
Write-Host "  $($installer.FullName)"
Write-Host "  $($installer.FullName).sig"
Write-Host "  $(Join-Path $root "$nsis/latest.json")"
