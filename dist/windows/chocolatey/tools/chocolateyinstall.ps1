$ErrorActionPreference = 'Stop'

$packageName = 'jarvy'
$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$version = 'VERSION_PLACEHOLDER'

$url64 = "https://github.com/bearbinary/jarvy/releases/download/v$version/jarvy-v$version-x86_64-pc-windows-msvc.zip"
$checksum64 = 'SHA256_PLACEHOLDER'

$packageArgs = @{
    packageName    = $packageName
    unzipLocation  = $toolsDir
    url64bit       = $url64
    checksum64     = $checksum64
    checksumType64 = 'sha256'
}

Install-ChocolateyZipPackage @packageArgs

# Create shim for jarvy.exe
$jarvy = Join-Path $toolsDir 'jarvy.exe'
if (Test-Path $jarvy) {
    Write-Host "Jarvy installed successfully!"
    Write-Host "Run 'jarvy --help' to get started."
}
