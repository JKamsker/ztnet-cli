use serde_json::{json, Value};

use crate::cli::{ConfigCommand, GlobalOpts, OutputFormat};
use crate::config::{self, Config};
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::host::normalize_host_input;
use crate::output;

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

			set_config_key(&mut cfg, &key, &args.value)?;
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

fn set_config_key(cfg: &mut Config, key: &str, value: &str) -> Result<(), CliError> {
	let parts: Vec<&str> = key.split('.').collect();
	match parts.as_slice() {
		["active_profile"] => {
			cfg.active_profile = Some(value.to_string());
			Ok(())
		}
		["profiles", profile, field] => {
			let p = cfg.profile_mut(profile);
			match *field {
				"host" => p.host = Some(normalize_host_input(value)?),
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
			let p = cfg.profile_mut(profile);
			match *field {
				"host" => p.host = None,
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
