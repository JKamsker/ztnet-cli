use std::env;
use std::time::Duration;

use crate::cli::{GlobalOpts, OutputFormat};
use crate::config::{Config, ConfigError};

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
) -> Result<EffectiveConfig, ConfigError> {
	let profile = global
		.profile
		.clone()
		.or_else(|| env::var("ZTNET_PROFILE").ok())
		.or_else(|| config.active_profile.clone())
		.unwrap_or_else(|| "default".to_string());

	let profile_cfg = config.profile(&profile);

	let host = global
		.host
		.clone()
		.or_else(|| env::var("ZTNET_HOST").ok())
		.or_else(|| env::var("API_ADDRESS").ok())
		.or_else(|| empty_to_none(profile_cfg.host.clone()))
		.unwrap_or_else(|| "http://localhost:3000".to_string());

	let token = global
		.token
		.clone()
		.or_else(|| env::var("ZTNET_API_TOKEN").ok())
		.or_else(|| env::var("ZTNET_TOKEN").ok())
		.or_else(|| empty_to_none(profile_cfg.token.clone()));

	let session_cookie = empty_to_none(profile_cfg.session_cookie.clone());
	let device_cookie = empty_to_none(profile_cfg.device_cookie.clone());

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

	let retries = global
		.retries
		.or(profile_cfg.retries)
		.unwrap_or(3);

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

