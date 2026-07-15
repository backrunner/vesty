$ErrorActionPreference = "Stop"

$repository = if ($env:VESTY_REPOSITORY) { $env:VESTY_REPOSITORY } else { "backrunner/vesty" }
$version = if ($env:VESTY_VERSION) { $env:VESTY_VERSION } else { "latest" }
$installDir = if ($env:VESTY_INSTALL_DIR) { $env:VESTY_INSTALL_DIR } else { Join-Path $HOME ".local\bin" }
$architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture

if ($architecture -ne [System.Runtime.InteropServices.Architecture]::X64) {
    throw "Vesty does not publish a Windows binary for architecture $architecture yet."
}

$asset = "vesty-x86_64-pc-windows-msvc.zip"
if ($version -eq "latest") {
    $downloadBase = "https://github.com/$repository/releases/latest/download"
} elseif ($version.StartsWith("v")) {
    $downloadBase = "https://github.com/$repository/releases/download/$version"
} else {
    throw "VESTY_VERSION must be latest or a v-prefixed release tag."
}

$temporaryDir = Join-Path ([System.IO.Path]::GetTempPath()) ("vesty-install-" + [System.Guid]::NewGuid())
New-Item -ItemType Directory -Path $temporaryDir | Out-Null

try {
    $archivePath = Join-Path $temporaryDir $asset
    $checksumsPath = Join-Path $temporaryDir "SHA256SUMS"
    Invoke-WebRequest "$downloadBase/$asset" -OutFile $archivePath
    Invoke-WebRequest "$downloadBase/SHA256SUMS" -OutFile $checksumsPath

    $checksumLine = Get-Content $checksumsPath | Where-Object { $_ -match "\s\*?$([regex]::Escape($asset))$" } | Select-Object -First 1
    if (-not $checksumLine) {
        throw "SHA256SUMS does not contain $asset."
    }

    $expectedChecksum = ($checksumLine -split "\s+")[0].ToLowerInvariant()
    $actualChecksum = (Get-FileHash -Path $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualChecksum -ne $expectedChecksum) {
        throw "Checksum verification failed for $asset."
    }

    Expand-Archive -Path $archivePath -DestinationPath $temporaryDir
    $archiveRoot = [System.IO.Path]::GetFileNameWithoutExtension($asset)
    $source = Join-Path $temporaryDir "$archiveRoot\vesty.exe"
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
    Copy-Item -Force $source (Join-Path $installDir "vesty.exe")

    Write-Host "Installed vesty to $(Join-Path $installDir 'vesty.exe')"
    if (($env:PATH -split ";") -notcontains $installDir) {
        Write-Host "Add $installDir to PATH before running vesty."
    }
} finally {
    Remove-Item -Recurse -Force $temporaryDir -ErrorAction SilentlyContinue
}
