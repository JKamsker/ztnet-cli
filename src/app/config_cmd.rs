use std::time::Duration;

use serde_json::{json, Value};

use crate::cli::{ConfigCommand, GlobalOpts, OutputFormat};
use crate::config::{self, Config};
use crate::context::canonical_host_key;
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::host::{api_base_candidates, normalize_host_input};
use crate::multi_base;
use crate::output;
use reqwest::StatusCode;
use url::Url;

use super::common::{
	load_config_store, opt_string, print_human_or_machine, redact_token, render_scalar,
};

pub(super) async fn run(global: &GlobalOpts, command: ConfigCommand) -> Result<(), CliError> {
	let (config_path, mut cfg) = load_config_store()?;
	let effective = resolve_effective_config(global, &cfg)?;

	match command {
		ConfigCommand::Path => {
			println!("{}", config_path.display());
			Ok(())
		}
		ConfigCommand::Get(args) => {
			let value = get_config_key(&cfg, &args.key)?;
			if matches!(effective.output, OutputFormat::Table) {
				println!("{}", render_scalar(&value));
				return Ok(());
			}
			output::print_value(&value, effective.output, global.no_color)?;
			Ok(())
		}
		ConfigCommand::Set(args) => {
			let key = if args.key == "host" {
				format!("profiles.{}.host", effective.profile)
			} else {
				args.key.clone()
			};

			let mut value = args.value.clone();
			if is_profile_host_key(&key) {
				let normalized = normalize_host_input(&value)?;
				if !args.no_validate && !global.dry_run {
					let timeout = effective.timeout.min(Duration::from_secs(5));
					let selected = select_valid_ztnet_host(&normalized, timeout).await?;
					if selected != normalized && !global.quiet {
						eprintln!("Host validated as '{selected}' (corrected from '{normalized}').");
					}
					value = selected;
				} else {
					value = normalized;
				}
			}

			set_config_key(&mut cfg, &key, &value, is_profile_host_key(&key))?;
			config::save_config(&config_path, &cfg)?;
			if !global.quiet {
				eprintln!("Set {}.", key);
			}
			Ok(())
		}
		ConfigCommand::Unset(args) => {
			unset_config_key(&mut cfg, &args.key)?;
			config::save_config(&config_path, &cfg)?;
			if !global.quiet {
				eprintln!("Unset {}.", args.key);
			}
			Ok(())
		}
		ConfigCommand::List => {
			let value = json!({
				"config_path": config_path.to_string_lossy(),
				"profile": effective.profile,
				"host": effective.host,
				"token": effective.token.as_deref().map(redact_token),
				"org": effective.org,
				"network": effective.network,
				"output": effective.output.to_string(),
				"timeout": humantime::format_duration(effective.timeout).to_string(),
				"retries": effective.retries,
			});
			print_human_or_machine(&value, effective.output, global.no_color)?;
			Ok(())
		}
		ConfigCommand::Context { command } => match command {
			crate::cli::ConfigContextCommand::Show => {
				let profile_cfg = cfg.profile(&effective.profile);
				let value = json!({
					"profile": effective.profile,
					"org": profile_cfg.default_org,
					"network": profile_cfg.default_network,
				});
				print_human_or_machine(&value, effective.output, global.no_color)?;
				Ok(())
			}
			crate::cli::ConfigContextCommand::Set(args) => {
				if args.org.is_none() && args.network.is_none() {
					return Err(CliError::InvalidArgument(
						"context set requires at least one of --org or --network".to_string(),
					));
				}
				let profile_cfg = cfg.profile_mut(&effective.profile);
				if let Some(org) = args.org {
					profile_cfg.default_org = Some(org);
				}
				if let Some(network) = args.network {
					profile_cfg.default_network = Some(network);
				}
				config::save_config(&config_path, &cfg)?;
				if !global.quiet {
					eprintln!("Context updated for profile '{}'.", effective.profile);
				}
				Ok(())
			}
			crate::cli::ConfigContextCommand::Clear => {
				let profile_cfg = cfg.profile_mut(&effective.profile);
				profile_cfg.default_org = None;
				profile_cfg.default_network = None;
				config::save_config(&config_path, &cfg)?;
				if !global.quiet {
					eprintln!("Context cleared for profile '{}'.", effective.profile);
				}
				Ok(())
			}
		},
	}
}

fn get_config_key(cfg: &Config, key: &str) -> Result<Value, CliError> {
	let parts: Vec<&str> = key.split('.').collect();
	match parts.as_slice() {
		["active_profile"] => Ok(cfg
			.active_profile
			.clone()
			.map(Value::String)
			.unwrap_or(Value::Null)),
		["profiles"] => Ok(serde_json::to_value(&cfg.profiles)?),
		["profiles", profile] => Ok(serde_json::to_value(cfg.profile(profile))?),
		["profiles", profile, field] => {
			let p = cfg.profile(profile);
			let v = match *field {
				"host" => opt_string(p.host),
				"token" => opt_string(p.token),
				"default_org" => opt_string(p.default_org),
				"default_network" => opt_string(p.default_network),
				"output" => p
					.output
					.map(|f| Value::String(f.to_string()))
					.unwrap_or(Value::Null),
				"timeout" => opt_string(p.timeout),
				"retries" => p
					.retries
					.map(|n| Value::Number(n.into()))
					.unwrap_or(Value::Null),
				_ => {
					return Err(CliError::InvalidArgument(format!(
						"unsupported key: {key}"
					)))
				}
			};
			Ok(v)
		}
		_ => Err(CliError::InvalidArgument(format!("unsupported key: {key}"))),
	}
}

fn set_config_key(
	cfg: &mut Config,
	key: &str,
	value: &str,
	skip_host_normalize: bool,
) -> Result<(), CliError> {
	let parts: Vec<&str> = key.split('.').collect();
	match parts.as_slice() {
		["active_profile"] => {
			cfg.active_profile = Some(value.to_string());
			Ok(())
		}
		["profiles", profile, field] => {
			match *field {
				"host" => {
					let normalized = if skip_host_normalize {
						value.to_string()
					} else {
						normalize_host_input(value)?
					};
					let host_key = canonical_host_key(&normalized)?;

					{
						let p = cfg.profile_mut(profile);
						p.host = Some(normalized);
					}

					let stale_keys: Vec<String> = cfg
						.host_defaults
						.iter()
						.filter(|(key, mapped_profile)| {
							mapped_profile.as_str() == *profile && key.as_str() != host_key.as_str()
						})
						.map(|(key, _)| key.clone())
						.collect();
					for key in stale_keys {
						cfg.host_defaults.remove(&key);
					}

					cfg.host_defaults.insert(host_key, profile.to_string());
				}
				other => {
					let p = cfg.profile_mut(profile);
					match other {
						"token" => p.token = Some(value.to_string()),
						"default_org" => p.default_org = Some(value.to_string()),
						"default_network" => p.default_network = Some(value.to_string()),
						"output" => {
							p.output = Some(parse_output_format(value)?);
						}
						"timeout" => {
							humantime::parse_duration(value).map_err(|_| {
								CliError::InvalidArgument(format!("invalid timeout value: {value}"))
							})?;
							p.timeout = Some(value.to_string());
						}
						"retries" => {
							let n = value.parse::<u32>().map_err(|_| {
								CliError::InvalidArgument(format!("invalid retries value: {value}"))
							})?;
							p.retries = Some(n);
						}
						_ => {
							return Err(CliError::InvalidArgument(format!(
								"unsupported key: {key}"
							)))
						}
					}
				}
			}
			Ok(())
		}
		_ => Err(CliError::InvalidArgument(format!("unsupported key: {key}"))),
	}
}

fn unset_config_key(cfg: &mut Config, key: &str) -> Result<(), CliError> {
	let parts: Vec<&str> = key.split('.').collect();
	match parts.as_slice() {
		["active_profile"] => {
			cfg.active_profile = None;
			Ok(())
		}
		["profiles", profile, field] => {
			match *field {
				"host" => {
					{
						let p = cfg.profile_mut(profile);
						p.host = None;
					}

					let keys_to_remove: Vec<String> = cfg
						.host_defaults
						.iter()
						.filter(|(_, mapped_profile)| mapped_profile.as_str() == *profile)
						.map(|(key, _)| key.clone())
						.collect();
					for key in keys_to_remove {
						cfg.host_defaults.remove(&key);
					}
				}
				other => {
					let p = cfg.profile_mut(profile);
					match other {
						"token" => p.token = None,
						"default_org" => p.default_org = None,
						"default_network" => p.default_network = None,
						"output" => p.output = None,
						"timeout" => p.timeout = None,
						"retries" => p.retries = None,
						_ => {
							return Err(CliError::InvalidArgument(format!(
								"unsupported key: {key}"
							)))
						}
					}
				}
			}
			Ok(())
		}
		_ => Err(CliError::InvalidArgument(format!("unsupported key: {key}"))),
	}
}

fn parse_output_format(value: &str) -> Result<crate::cli::OutputFormat, CliError> {
	let normalized = value.trim().to_ascii_lowercase();
	match normalized.as_str() {
		"table" => Ok(crate::cli::OutputFormat::Table),
		"json" => Ok(crate::cli::OutputFormat::Json),
		"yaml" | "yml" => Ok(crate::cli::OutputFormat::Yaml),
		"raw" => Ok(crate::cli::OutputFormat::Raw),
		_ => Err(CliError::InvalidArgument(format!(
			"invalid output format: {value}"
		))),
	}
}

fn is_profile_host_key(key: &str) -> bool {
	let mut parts = key.split('.');
	parts.next() == Some("profiles")
		&& parts.next().is_some()
		&& parts.next() == Some("host")
		&& parts.next().is_none()
}

async fn select_valid_ztnet_host(base: &str, timeout: Duration) -> Result<String, CliError> {
	let candidates = api_base_candidates(base);

	let client = reqwest::Client::builder().timeout(timeout).build()?;

	let mut last_error = None;
	for candidate in &candidates {
		match probe_ztnet_instance(&client, candidate).await {
			Ok(()) => return Ok(candidate.clone()),
			Err(err) => last_error = Some(err),
		}
	}

	let detail = last_error.unwrap_or_else(|| "unknown error".to_string());
	Err(CliError::InvalidArgument(format!(
		"host did not look like a ZTNet instance (tried: {}): {detail} (pass --no-validate to save anyway)",
		candidates.join(", ")
	)))
}

async fn probe_ztnet_instance(client: &reqwest::Client, base: &str) -> Result<(), String> {
	// `api_base_candidates` may return either the bare host or the "/api" base; the probe paths are
	// relative to the chosen base to avoid joining "api/..." onto an already "/api"-suffixed URL.
	let base_has_api_suffix = base.trim_end_matches('/').ends_with("/api");

	let csrf_path = if base_has_api_suffix {
		"auth/csrf"
	} else {
		"api/auth/csrf"
	};
	let csrf_url = build_url_from_base(base, csrf_path).map_err(|e| e.to_string())?;
	let resp = client
		.get(csrf_url)
		.header("accept", "application/json")
		.send()
		.await
		.map_err(|err| format!("GET /api/auth/csrf request failed: {err}"))?;

	let status = resp.status();
	if !status.is_success() {
		return Err(format!("GET /api/auth/csrf returned {status}"));
	}

	let value = resp
		.json::<serde_json::Value>()
		.await
		.map_err(|err| format!("GET /api/auth/csrf did not return JSON: {err}"))?;
	let csrf = value
		.get("csrfToken")
		.and_then(|v| v.as_str())
		.unwrap_or("")
		.trim();
	if csrf.is_empty() {
		return Err("GET /api/auth/csrf missing csrfToken".to_string());
	}

	let network_path = if base_has_api_suffix {
		"v1/network"
	} else {
		"api/v1/network"
	};
	let api_url = build_url_from_base(base, network_path).map_err(|e| e.to_string())?;
	let resp = client
		.get(api_url)
		// Some reverse proxies / deployments return 5xx when no token is provided at all.
		// Sending an intentionally invalid token should still yield 401/403 if the endpoint exists.
		.header("x-ztnet-auth", "invalid")
		.header("accept", "application/json")
		.send()
		.await
		.map_err(|err| format!("GET /api/v1/network request failed: {err}"))?;

	match resp.status() {
		StatusCode::OK | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Ok(()),
		status => Err(format!("GET /api/v1/network returned {status}")),
	}
}

fn build_url_from_base(base: &str, path: &str) -> Result<Url, CliError> {
	multi_base::parse_normalize_and_join_url(base, path)
}
