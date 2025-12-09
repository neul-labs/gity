$ErrorActionPreference = 'Stop'

$packageName = 'gity'
$version = '0.1.0'
$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"

$url64 = "https://github.com/neul-labs/gity/releases/download/v$version/gity-$version-x86_64-pc-windows-msvc.zip"
$checksum64 = 'PLACEHOLDER_SHA256_WINDOWS_X64'

$packageArgs = @{
  packageName    = $packageName
  unzipLocation  = $toolsDir
  url64bit       = $url64
  checksum64     = $checksum64
  checksumType64 = 'sha256'
}

Install-ChocolateyZipPackage @packageArgs

# Add to PATH
$binPath = Join-Path $toolsDir 'gity.exe'
Install-BinFile -Name 'gity' -Path $binPath
