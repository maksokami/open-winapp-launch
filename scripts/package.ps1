[CmdletBinding()]
param(
    [string]$Iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $status = git status --porcelain --untracked-files=normal
    if ($LASTEXITCODE -ne 0) { throw 'Packaging requires a Git checkout' }
    if ($status) { throw 'Commit all release inputs before packaging; source and binaries must match' }
    $metadata = cargo metadata --locked --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $version = $metadata.packages[0].version
    if ($version -notmatch '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') {
        throw 'Packaging supports stable MAJOR.MINOR.PATCH versions only'
    }
    $exe = 'target\x86_64-pc-windows-msvc\release\open-winapp-launcher.exe'
    if (!(Test-Path $exe -PathType Leaf)) { throw "Build $exe first" }
    if (!(Test-Path $Iscc -PathType Leaf)) { throw "Inno Setup 6 compiler not found: $Iscc" }
    if (Test-Path dist) {
        throw 'dist already exists. Move or remove the previous dist directory before packaging to avoid mixing releases.'
    }
    New-Item -ItemType Directory dist | Out-Null
    $portable = "dist\open-winapp-launcher-$version-windows-x64"
    New-Item -ItemType Directory $portable | Out-Null
    cargo bundle-licenses --format json --output dist\THIRD_PARTY_LICENSES.json
    if ($LASTEXITCODE -ne 0) { throw 'Dependency license inventory failed' }
    $source = "dist\open-winapp-launcher-$version-source"
    git archive --format=zip --output=dist\source-from-git.zip HEAD
    if ($LASTEXITCODE -ne 0) { throw 'Could not archive committed source; commit all release inputs first' }
    Expand-Archive dist\source-from-git.zip -DestinationPath $source
    Remove-Item dist\source-from-git.zip
    New-Item -ItemType Directory "$source\.cargo" -Force | Out-Null
    $vendorConfig = cargo vendor --locked --versioned-dirs "$source\vendor"
    if ($LASTEXITCODE -ne 0) { throw 'Could not vendor dependency source for GPL distribution' }
    $vendorConfig = $vendorConfig -replace 'directory = "[^"]+"', 'directory = "vendor"'
    $vendorConfig | Set-Content "$source\.cargo\config.toml" -Encoding utf8
    Compress-Archive -Path "$source\*" -DestinationPath "$source.zip"
    $vendor = (Resolve-Path "$source\vendor").Path
    $notices = 'dist\THIRD_PARTY_NOTICES'
    New-Item -ItemType Directory $notices | Out-Null
    Get-ChildItem $vendor -Recurse -File |
        Where-Object {
            $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|AUTHORS|COPYRIGHT)([._-].*)?$' -or
            $_.FullName.Substring($vendor.Length) -match '[\\/](LICENSES?|LICENCES?)[\\/]'
        } |
        ForEach-Object {
            $relative = [System.IO.Path]::GetRelativePath($vendor, $_.FullName)
            $destination = Join-Path $notices $relative
            New-Item -ItemType Directory (Split-Path $destination -Parent) -Force | Out-Null
            Copy-Item $_.FullName $destination
        }
    Copy-Item LICENSE "$notices\GPL-3.0-only.txt"
    Copy-Item $exe "$portable\open-winapp-launcher.exe"
    Copy-Item LICENSE,README.md,THIRD_PARTY.md,dist\THIRD_PARTY_LICENSES.json $portable
    Copy-Item $notices "$portable\THIRD_PARTY_NOTICES" -Recurse
    Copy-Item examples "$portable\examples" -Recurse
    Copy-Item docs "$portable\docs" -Recurse
    Compress-Archive -Path "$portable\*" -DestinationPath "$portable.zip"
    & $Iscc "/DAppVersion=$version" 'installer\open-winapp-launcher.iss'
    if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed with exit code $LASTEXITCODE" }
    Get-ChildItem dist -File |
        Where-Object { $_.Extension -in '.exe', '.zip', '.json' } |
        Sort-Object Name |
        ForEach-Object {
            $hash = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            "$hash  $($_.Name)"
        } | Set-Content dist\SHA256SUMS.txt -Encoding ascii
}
finally {
    Pop-Location
}
