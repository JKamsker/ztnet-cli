param(
  [string]$Repo = "JKamsker/ztnet-cli"
)

$ErrorActionPreference = "Stop"

$secretName = "CHOCO_API_KEY"
$value = $env:CHOCO_API_KEY

if ([string]::IsNullOrWhiteSpace($value)) {
  throw "CHOCO_API_KEY is empty. Set it first (e.g. `$env:CHOCO_API_KEY = '...') and re-run."
}

$value | gh secret set $secretName --repo $Repo --app actions
Write-Host "Set $secretName for $Repo"

