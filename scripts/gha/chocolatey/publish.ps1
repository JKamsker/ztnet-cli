$ErrorActionPreference = 'Stop'

$version = $env:ZTNET_VERSION
if ([string]::IsNullOrWhiteSpace($version)) {
  throw "ZTNET_VERSION env var is required"
}

$version = $version.Trim()
$tag = "v$version"

$shaUrl = "https://github.com/JKamsker/ztnet-cli/releases/download/$tag/ztnet-$version-x86_64-pc-windows-msvc.zip.sha256"
$shaLine = (Invoke-RestMethod $shaUrl).Trim()
$hash = $shaLine.Split(' ')[0]
if ([string]::IsNullOrWhiteSpace($hash)) {
  throw "Failed to determine SHA256 for $shaUrl"
}

Remove-Item -Recurse -Force .tmp_choco -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force .tmp_choco/pkg | Out-Null

Copy-Item -Recurse -Force packaging/chocolatey/ztnet/* .tmp_choco/pkg

foreach ($path in @(
  ".tmp_choco/pkg/tools/chocolateyinstall.ps1",
  ".tmp_choco/pkg/tools/VERIFICATION.txt"
)) {
  $content = Get-Content -Raw $path
  $content = $content -replace "__CHECKSUM__", $hash
  Set-Content -Path $path -Value $content -NoNewline
}

New-Item -ItemType Directory -Force .tmp_choco/out | Out-Null
choco pack .tmp_choco/pkg/ztnet.nuspec --version $version --outputdirectory .tmp_choco/out --no-progress

$nupkg = Get-ChildItem .tmp_choco/out -Filter "ztnet.$version.nupkg" | Select-Object -First 1
if (-not $nupkg) {
  throw "Expected nupkg not found in .tmp_choco/out"
}

& choco push $nupkg.FullName --source https://push.chocolatey.org/ --api-key $env:CHOCO_API_KEY --no-progress 2>&1 | Tee-Object -Variable chocoPushOutput
$pushExitCode = $LASTEXITCODE
if ($pushExitCode -ne 0) {
  $pushText = ($chocoPushOutput -join "`n")
  if ($pushText -match "previous version in a submitted state") {
    Write-Host "Chocolatey push blocked (previous version pending approval). Skipping."
    exit 0
  }
  throw "choco push failed with exit code $pushExitCode"
}

