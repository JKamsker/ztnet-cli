$ErrorActionPreference = 'Stop'

$packageName = $env:ChocolateyPackageName
Uninstall-BinFile $packageName

