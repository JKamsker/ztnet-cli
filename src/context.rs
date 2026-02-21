use std::env;
use std::time::Duration;

use crate::cli::{GlobalOpts, OutputFormat};
use crate::config::{Config, ConfigError};
use crate::error::CliError;
use url::Url;

#[derive(Debug, Clone)]
pub struct EffectiveConfig {
	pub profile: String,
	pub host: String,
	pub token: Option<String>,
	pub session_cookie: Option<String>,
	pub device_cookie: Option<String>,
	pub org: Option<String>,
	pub network: Option<String>,
	pub output: OutputFormat,
	pub timeout: Duration,
	pub retries: u32,
}

pub fn resolve_effective_config(
	global: &GlobalOpts,
	config: &Config,
) -> Result<EffectiveConfig, CliError> {
	let explicit_profile = global
		.profile
		.clone()
		.or_else(|| env::var("ZTNET_PROFILE").ok());

	let explicit_host = global
		.host
		.clone()
		.or_else(|| env::var("ZTNET_HOST").ok())
		.or_else(|| env::var("API_ADDRESS").ok());

	let profile = if let Some(profile) = explicit_profile.clone() {
		profile
	} else if let Some(ref host) = explicit_host {
		let host_key = canonical_host_key(host)?;
		select_profile_for_host(&host_key, config)?
			.unwrap_or_else(|| config.active_profile.clone().unwrap_or_else(|| "default".to_string()))
	} else {
		config
			.active_profile
			.clone()
			.unwrap_or_else(|| "default".to_string())
	};

	let profile_cfg = config.profile(&profile);

	let host = if let Some(host) = explicit_host {
		if explicit_profile.is_some() {
			if let Some(profile_host) = empty_to_none(profile_cfg.host.clone()) {
				let profile_key = canonical_host_key(&profile_host)?;
				let target_key = canonical_host_key(&host)?;
				if profile_key != target_key {
					return Err(CliError::InvalidArgument(format!(
						"profile '{profile}' is configured for '{profile_host}', but the target host is '{host}'",
					)));
				}
			}
		}
		host
	} else {
		empty_to_none(profile_cfg.host.clone()).unwrap_or_else(|| "http://localhost:3000".to_string())
	};

	let target_host_key = canonical_host_key(&host)?;
	let profile_host_key = canonical_host_key_opt(profile_cfg.host.as_deref());
	let profile_host_matches = profile_host_key.as_deref() == Some(&target_host_key);

	let token_override = global
		.token
		.clone()
		.or_else(|| env::var("ZTNET_API_TOKEN").ok())
		.or_else(|| env::var("ZTNET_TOKEN").ok());

	let token = if token_override.is_some() {
		token_override
	} else if profile_host_matches {
		empty_to_none(profile_cfg.token.clone())
	} else {
		None
	};

	let session_cookie = profile_host_matches
		.then(|| empty_to_none(profile_cfg.session_cookie.clone()))
		.flatten();
	let device_cookie = profile_host_matches
		.then(|| empty_to_none(profile_cfg.device_cookie.clone()))
		.flatten();

	let org = global
		.org
		.clone()
		.or_else(|| empty_to_none(profile_cfg.default_org.clone()));

	let network = global
		.network
		.clone()
		.or_else(|| empty_to_none(profile_cfg.default_network.clone()));

	let output = if global.json {
		OutputFormat::Json
	} else if let Some(output) = global.output {
		output
	} else if let Ok(value) = env::var("ZTNET_OUTPUT") {
		parse_output_format(&value)?
	} else {
		profile_cfg.output.unwrap_or(OutputFormat::Table)
	};

	let timeout_str = global
		.timeout
		.clone()
		.or_else(|| empty_to_none(profile_cfg.timeout.clone()))
		.unwrap_or_else(|| "30s".to_string());

	let timeout = humantime::parse_duration(&timeout_str)
		.map_err(|_| ConfigError::InvalidTimeout(timeout_str))?;

	let retries = global.retries.or(profile_cfg.retries).unwrap_or(3);

	Ok(EffectiveConfig {
		profile,
		host,
		token,
		session_cookie,
		device_cookie,
		org,
		network,
		output,
		timeout,
		retries,
	})
}

fn parse_output_format(value: &str) -> Result<OutputFormat, ConfigError> {
	let normalized = value.trim().to_ascii_lowercase();
	match normalized.as_str() {
		"table" => Ok(OutputFormat::Table),
		"json" => Ok(OutputFormat::Json),
		"yaml" | "yml" => Ok(OutputFormat::Yaml),
		"raw" => Ok(OutputFormat::Raw),
		_ => Err(ConfigError::InvalidOutputFormat(value.to_string())),
	}
}

fn empty_to_none(value: Option<String>) -> Option<String> {
	match value {
		Some(v) if v.trim().is_empty() => None,
		other => other,
	}
}

fn canonical_host_key(raw: &str) -> Result<String, CliError> {
	let url = Url::parse(raw.trim())
		.map_err(|err| CliError::InvalidArgument(format!("invalid host url: {err}")))?;

	let scheme = url.scheme().to_ascii_lowercase();
	let host = url.host().ok_or_else(|| {
		CliError::InvalidArgument(format!("invalid host url: missing hostname in '{raw}'"))
	})?;

	let host_part = match host {
		url::Host::Domain(domain) => domain.to_ascii_lowercase(),
		url::Host::Ipv4(ip) => ip.to_string(),
		url::Host::Ipv6(ip) => format!("[{ip}]"),
	};

	let default_port = match scheme.as_str() {
		"http" => Some(80),
		"https" => Some(443),
		_ => None,
	};

	let port = url.port();
	let include_port = match (port, default_port) {
		(Some(p), Some(d)) => p != d,
		(Some(_), None) => true,
		(None, _) => false,
	};

	if include_port {
		Ok(format!("{scheme}://{host_part}:{}", port.expect("include_port implies Some")))
	} else {
		Ok(format!("{scheme}://{host_part}"))
	}
}

fn canonical_host_key_opt(raw: Option<&str>) -> Option<String> {
	let raw = raw?.trim();
	if raw.is_empty() {
		return None;
	}
	Url::parse(raw)
		.ok()
		.and_then(|url| {
			let scheme = url.scheme().to_ascii_lowercase();
			let host = url.host()?;
			let host_part = match host {
				url::Host::Domain(domain) => domain.to_ascii_lowercase(),
				url::Host::Ipv4(ip) => ip.to_string(),
				url::Host::Ipv6(ip) => format!("[{ip}]"),
			};

			let default_port = match scheme.as_str() {
				"http" => Some(80),
				"https" => Some(443),
				_ => None,
			};

			let port = url.port();
			let include_port = match (port, default_port) {
				(Some(p), Some(d)) => p != d,
				(Some(_), None) => true,
				(None, _) => false,
			};

			Some(if include_port {
				format!("{scheme}://{host_part}:{}", port.expect("include_port implies Some"))
			} else {
				format!("{scheme}://{host_part}")
			})
		})
}

fn select_profile_for_host(host_key: &str, config: &Config) -> Result<Option<String>, CliError> {
	if let Some(profile) = config.host_defaults.get(host_key).cloned() {
		if !config.profiles.contains_key(&profile) {
			return Err(CliError::InvalidArgument(format!(
				"host_defaults maps '{host_key}' to unknown profile '{profile}'"
			)));
		}

		let cfg = config.profile(&profile);
		let profile_host_key = canonical_host_key_opt(cfg.host.as_deref());
		if profile_host_key.as_deref() != Some(host_key) {
			return Err(CliError::InvalidArgument(format!(
				"host_defaults for '{host_key}' points to profile '{profile}', but that profile's host does not match"
			)));
		}

		return Ok(Some(profile));
	}

	for name in config.profiles.keys() {
		let cfg = config.profile(name);
		let profile_host_key = canonical_host_key_opt(cfg.host.as_deref());
		if profile_host_key.as_deref() == Some(host_key) {
			return Ok(Some(name.clone()));
		}
	}

	Ok(None)
}

