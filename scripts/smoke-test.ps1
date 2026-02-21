param(
	[string]$BaseUrl = $(if ($env:ZTNET_HOST) { $env:ZTNET_HOST } else { "http://localhost:3000" }),
	[string]$Email = $env:ZTNET_SMOKE_EMAIL,
	[string]$Password = $env:ZTNET_SMOKE_PASSWORD,
	[string]$Name = $(if ($env:ZTNET_SMOKE_NAME) { $env:ZTNET_SMOKE_NAME } else { "ZTNet CLI" }),
	[string]$Bin = "target\\debug\\ztnet.exe"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $Bin)) {
	cargo build | Out-Host
}

Write-Host "Waiting for ZTNet at $BaseUrl ..."
$deadline = (Get-Date).AddMinutes(3)
while ($true) {
	try {
		$status = (& curl.exe -s -o NUL -w "%{http_code}" $BaseUrl).Trim()
		if ($LASTEXITCODE -eq 0) {
			$code = [int]$status
			if ($code -ge 200 -and $code -lt 500) {
				break
			}
		}
	} catch {
		if ((Get-Date) -gt $deadline) {
			throw "ZTNet did not become ready at $BaseUrl within 3 minutes."
		}
		Start-Sleep -Seconds 2
	}
}

# Bootstrap user/token once (only works on a fresh DB).
if ($Email -and $Password) {
	try {
		Write-Host "Attempting bootstrap user creation (may fail if already initialized) ..."
		& $Bin --host $BaseUrl user create --email $Email --password $Password --name $Name --generate-api-token --store-token --no-auth | Out-Host
		if ($LASTEXITCODE -ne 0) {
			throw "user create failed with exit code $LASTEXITCODE"
		}
	} catch {
		Write-Host "Bootstrap skipped/failed: $($_.Exception.Message)"
	}
}

Write-Host "Auth test ..."
& $Bin --host $BaseUrl auth test | Out-Host
if ($LASTEXITCODE -ne 0) {
	throw "auth test failed with exit code $LASTEXITCODE"
}

$netName = "cli-smoke-" + (Get-Date -Format "yyyyMMdd-HHmmss")
Write-Host "Creating network $netName ..."
$createdJson = (& $Bin --json --host $BaseUrl network create --name $netName)
if ($LASTEXITCODE -ne 0) {
	throw "network create failed with exit code $LASTEXITCODE"
}
$created = ($createdJson | ConvertFrom-Json)
$netId = $created.id
Write-Host "Created network id: $netId"

Write-Host "Network get ..."
& $Bin --host $BaseUrl network get $netId | Out-Host
if ($LASTEXITCODE -ne 0) {
	throw "network get failed with exit code $LASTEXITCODE"
}

Write-Host "Export hosts (json) ..."
& $Bin --host $BaseUrl export hosts $netId --zone example.test --format json | Out-Host
if ($LASTEXITCODE -ne 0) {
	throw "export hosts failed with exit code $LASTEXITCODE"
}

Write-Host "Smoke test complete."
