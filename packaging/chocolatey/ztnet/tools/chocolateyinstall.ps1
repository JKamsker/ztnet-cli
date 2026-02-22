$ErrorActionPreference = 'Stop'

$packageName = $env:ChocolateyPackageName
$toolsDir = Split-Path $MyInvocation.MyCommand.Definition

$version = $env:ChocolateyPackageVersion
$url64 = "https://github.com/JKamsker/ztnet-cli/releases/download/v$version/ztnet-$version-x86_64-pc-windows-msvc.zip"

$packageArgs = @{
  packageName   = $packageName
  unzipLocation = $toolsDir
  url64bit      = $url64
  checksum64    = "__CHECKSUM__"
  checksumType64 = 'sha256'
}

Install-ChocolateyZipPackage @packageArgs

$exePath = Join-Path $toolsDir 'ztnet.exe'
Install-BinFile -Name 'ztnet' -Path $exePath

