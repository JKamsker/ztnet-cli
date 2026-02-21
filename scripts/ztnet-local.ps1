param(
	[ValidateSet("up", "down", "ps", "logs")]
	[string]$Action = "up",

	[switch]$Volumes,
	[switch]$Follow,

	[string]$ComposeFile = "external/ztnet/docker-compose.yml"
)

$ErrorActionPreference = "Stop"

$composeArgs = @("-f", $ComposeFile)

switch ($Action) {
	"up" {
		docker compose @composeArgs up -d
	}
	"down" {
		if ($Volumes) {
			docker compose @composeArgs down -v
		} else {
			docker compose @composeArgs down
		}
	}
	"ps" {
		docker compose @composeArgs ps
	}
	"logs" {
		if ($Follow) {
			docker compose @composeArgs logs -f
		} else {
			docker compose @composeArgs logs
		}
	}
}

