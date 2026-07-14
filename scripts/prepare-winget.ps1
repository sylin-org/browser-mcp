# Prepare the three-file microsoft/winget-pkgs submission tree from Ghostlight's canonical
# release-filled manifest, then validate it with the installed Windows Package Manager.
[CmdletBinding()]
param(
    [string] $Source = (Join-Path $PSScriptRoot '..\packaging\winget\Sylin.Ghostlight.yaml'),
    [string] $OutputRoot = (Join-Path ([System.IO.Path]::GetTempPath()) 'ghostlight-winget-pkgs'),
    [switch] $SkipValidation
)

$ErrorActionPreference = 'Stop'

$sourcePath = (Resolve-Path -LiteralPath $Source).Path
$text = Get-Content -Raw -LiteralPath $sourcePath
$documents = [regex]::Split($text, '(?m)^---\s*$') |
    ForEach-Object { ([regex]::Replace($_, '(?m)^\s*#.*(?:\r?\n|$)', '')).Trim() } |
    Where-Object { $_ }

if ($documents.Count -ne 3) {
    throw "Expected exactly three YAML documents in $sourcePath; found $($documents.Count)."
}

$byType = @{}
foreach ($document in $documents) {
    $typeMatch = [regex]::Match($document, '(?m)^ManifestType:\s*(?<value>\S+)\s*$')
    if (-not $typeMatch.Success) { throw 'A YAML document has no ManifestType.' }
    $type = $typeMatch.Groups['value'].Value
    if ($byType.ContainsKey($type)) { throw "Duplicate ManifestType '$type'." }
    $byType[$type] = $document
}

$expectedTypes = @('version', 'installer', 'defaultLocale')
foreach ($type in $expectedTypes) {
    if (-not $byType.ContainsKey($type)) { throw "Missing ManifestType '$type'." }
}

$versions = foreach ($document in $documents) {
    $match = [regex]::Match($document, '(?m)^PackageVersion:\s*(?<value>\S+)\s*$')
    if (-not $match.Success) { throw 'A YAML document has no PackageVersion.' }
    $match.Groups['value'].Value
}
$version = $versions[0]
if (($versions | Select-Object -Unique).Count -ne 1) {
    throw "PackageVersion differs across documents: $($versions -join ', ')."
}
if ($documents | Where-Object { $_ -notmatch '(?m)^PackageIdentifier:\s*Sylin\.Ghostlight\s*$' }) {
    throw 'Every document must use PackageIdentifier Sylin.Ghostlight.'
}

$installer = $byType['installer']
if ($installer -notmatch '(?m)^\s*InstallerSha256:\s*[0-9a-fA-F]{64}\s*$') {
    throw 'InstallerSha256 must be a filled 64-character hexadecimal digest.'
}
if ($installer -notmatch [regex]::Escape("/v$version/ghostlight-v$version-x86_64-pc-windows-msvc.zip")) {
    throw "InstallerUrl does not identify the v$version Windows archive."
}

$destination = Join-Path $OutputRoot "manifests\s\Sylin\Ghostlight\$version"
New-Item -ItemType Directory -Force -Path $destination | Out-Null

$files = [ordered]@{
    'Sylin.Ghostlight.yaml' = 'version'
    'Sylin.Ghostlight.installer.yaml' = 'installer'
    'Sylin.Ghostlight.locale.en-US.yaml' = 'defaultLocale'
}
foreach ($entry in $files.GetEnumerator()) {
    $content = $byType[$entry.Value].Trim() + "`n"
    Set-Content -LiteralPath (Join-Path $destination $entry.Key) -Value $content -NoNewline
}

if (-not $SkipValidation) {
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        throw 'winget is not installed; rerun with -SkipValidation only if another validator will run.'
    }
    & winget validate --manifest $destination --disable-interactivity
    if ($LASTEXITCODE -ne 0) { throw "winget validate failed with exit code $LASTEXITCODE." }
}

Write-Output $destination
