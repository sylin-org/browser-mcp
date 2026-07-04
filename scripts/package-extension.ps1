<#
.SYNOPSIS
  Packages the Ghostlight Browser extension into a Chrome-Web-Store-ready zip.

.DESCRIPTION
  Reads the version out of extension/manifest.json, stages the store-relevant
  extension files into a temp folder (excluding local-install/dev-only files),
  and zips the STAGED folder's contents -- not the folder itself -- to
  dist/ghostlight-extension-v<version>.zip, so manifest.json sits at the zip
  root as the Chrome Web Store requires.

.PARAMETER Version
  Overrides the version used in the output filename. Defaults to the version
  field read from extension/manifest.json.

.EXAMPLE
  pwsh -File scripts\package-extension.ps1

.EXAMPLE
  pwsh -File scripts\package-extension.ps1 -Version 0.2.0
#>
param(
  [string]$Version
)

$ErrorActionPreference = 'Stop'

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$ExtensionDir = Join-Path $RepoRoot 'extension'
$ManifestPath = Join-Path $ExtensionDir 'manifest.json'
$DistDir = Join-Path $RepoRoot 'dist'

if (-not (Test-Path $ManifestPath)) {
  throw "manifest.json not found at $ManifestPath"
}
$manifest = Get-Content $ManifestPath -Raw | ConvertFrom-Json
if (-not $Version) { $Version = $manifest.version }
if (-not $Version) { throw "No version found in $ManifestPath and no -Version override given." }

# Dev-only files not wanted in the store package: native-messaging-host.json is a
# local-install template, README.md is developer-facing docs.
$ExcludeRelativePaths = @('native-messaging-host.json', 'README.md')

$StageDir = Join-Path $env:TEMP "ghostlight-extension-stage-$([guid]::NewGuid())"
New-Item -ItemType Directory -Path $StageDir | Out-Null

try {
  Get-ChildItem -Path $ExtensionDir -Recurse -File | ForEach-Object {
    [pscustomobject]@{
      Full     = $_.FullName
      Relative = $_.FullName.Substring($ExtensionDir.Length).TrimStart('\', '/')
    }
  } | Where-Object { $ExcludeRelativePaths -notcontains $_.Relative } | ForEach-Object {
    $target = Join-Path $StageDir $_.Relative
    New-Item -ItemType Directory -Path (Split-Path $target) -Force | Out-Null
    Copy-Item -Path $_.Full -Destination $target
  }

  New-Item -ItemType Directory -Path $DistDir -Force | Out-Null

  $ZipPath = Join-Path $DistDir "ghostlight-extension-v$Version.zip"
  if (Test-Path $ZipPath) { Remove-Item -Path $ZipPath -Force }

  # Zip the staged folder's CONTENTS (trailing \*), not the folder itself, so
  # manifest.json lands at the zip root -- the Chrome Web Store rejects a
  # manifest nested under a subfolder.
  Compress-Archive -Path (Join-Path $StageDir '*') -DestinationPath $ZipPath -Force
}
finally {
  Remove-Item -Path $StageDir -Recurse -Force -ErrorAction SilentlyContinue
}

$ZipPath = (Resolve-Path $ZipPath).Path
$sizeKb = [math]::Round((Get-Item $ZipPath).Length / 1KB, 1)
Write-Host "Packaged: $ZipPath ($sizeKb KB)" -ForegroundColor Green
