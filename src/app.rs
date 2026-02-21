use std::io::{self, Read};
use std::path::PathBuf;

use clap::CommandFactory;
use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{
	AuthCommand, Cli, Command, ConfigCommand, GlobalOpts, OrgCommand, OutputFormat, UserCommand,
};
use crate::config::{self, Config};
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::http::HttpClient;
use crate::output;

pub async fn run(cli: Cli) -> Result<(), CliError> {
	let Cli { global, command } = cli;

	match command {
		Command::Completion(args) => {
			let mut cmd = Cli::command();
			clap_complete::generate(args.shell, &mut cmd, "ztnet", &mut std::io::stdout());
			Ok(())
		}
		Command::Auth { command } => run_auth(&global, command).await,
		Command::Config { command } => run_config(&global, command).await,
		Command::User { command } => run_user(&global, command).await,
		Command::Org { command } => run_org(&global, command).await,
		_ => Err(CliError::Unimplemented("command")),
	}
}

async fn run_auth(global: &GlobalOpts, command: AuthCommand) -> Result<(), CliError> {
	let (config_path, mut cfg) = load_config_store()?;
	let effective = resolve_effective_config(global, &cfg)?;

	match command {
		AuthCommand::SetToken(args) => {
			if args.stdin && args.token.is_some() {
				return Err(CliError::InvalidArgument(
					"cannot combine --stdin with a positional TOKEN".to_string(),
				));
			}

			let profile = args.profile.unwrap_or_else(|| effective.profile.clone());
			let token = if args.stdin {
				read_stdin_trimmed()?
			} else {
				args.token.ok_or_else(|| {
					CliError::InvalidArgument("missing TOKEN (or pass --stdin)".to_string())
				})?
			};

			if token.is_empty() {
				return Err(CliError::InvalidArgument("token cannot be empty".to_string()));
			}

			cfg.profile_mut(&profile).token = Some(token);
			config::save_config(&config_path, &cfg)?;

			if !global.quiet {
				eprintln!("Token saved to profile '{profile}'.");
			}
			Ok(())
		}
		AuthCommand::UnsetToken(args) => {
			let profile = args.profile.unwrap_or_else(|| effective.profile.clone());
			cfg.profile_mut(&profile).token = None;
			config::save_config(&config_path, &cfg)?;

			if !global.quiet {
				eprintln!("Token removed from profile '{profile}'.");
			}
			Ok(())
		}
		AuthCommand::Show => {
			let value = json!({
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
		AuthCommand::Test(args) => {
			let path = if args.org.is_some() { "/api/v1/org" } else { "/api/v1/network" };

			let client = HttpClient::new(
				&effective.host,
				effective.token.clone(),
				effective.timeout,
				effective.retries,
				global.dry_run,
			)?;

			let response = client
				.request_json(Method::GET, path, None, Default::default(), true)
				.await?;

			if matches!(effective.output, OutputFormat::Table) {
				println!("OK");
				return Ok(());
			}

			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
		AuthCommand::Profiles { command } => match command {
			crate::cli::AuthProfilesCommand::List => {
				let active = cfg.active_profile.clone();
				let profiles: Vec<String> = cfg.profiles.keys().cloned().collect();
				let value = json!({ "active_profile": active, "profiles": profiles });
				print_human_or_machine(&value, effective.output, global.no_color)?;
				Ok(())
			}
			crate::cli::AuthProfilesCommand::Use(args) => {
				cfg.active_profile = Some(args.name.clone());
				cfg.profile_mut(&args.name);
				config::save_config(&config_path, &cfg)?;

				if !global.quiet {
					eprintln!("Active profile set to '{}'.", args.name);
				}
				Ok(())
			}
		},
	}
}

async fn run_config(global: &GlobalOpts, command: ConfigCommand) -> Result<(), CliError> {
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
			set_config_key(&mut cfg, &args.key, &args.value)?;
			config::save_config(&config_path, &cfg)?;
			if !global.quiet {
				eprintln!("Set {}.", args.key);
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

async fn run_user(global: &GlobalOpts, command: UserCommand) -> Result<(), CliError> {
	let (config_path, mut cfg) = load_config_store()?;
	let effective = resolve_effective_config(global, &cfg)?;

	match command {
		UserCommand::Create(args) => {
			let mut body = serde_json::Map::new();
			body.insert("email".to_string(), Value::String(args.email));
			body.insert("password".to_string(), Value::String(args.password));
			body.insert("name".to_string(), Value::String(args.name));

			if let Some(expires_at) = args.expires_at {
				body.insert("expiresAt".to_string(), Value::String(expires_at));
			}

			if args.generate_api_token {
				body.insert("generateApiToken".to_string(), Value::Bool(true));
			}

			let client = HttpClient::new(
				&effective.host,
				effective.token.clone(),
				effective.timeout,
				effective.retries,
				global.dry_run,
			)?;

			let include_auth = !args.no_auth && effective.token.is_some();
			let response = client
				.request_json(
					Method::POST,
					"/api/v1/user",
					Some(Value::Object(body)),
					Default::default(),
					include_auth,
				)
				.await?;

			let api_token = response
				.get("apiToken")
				.and_then(|v| v.as_str())
				.map(str::to_string);

			if (args.store_token || args.print_token) && api_token.is_none() {
				return Err(CliError::InvalidArgument(
					"server did not return an apiToken (try --generate-api-token)".to_string(),
				));
			}

			if args.store_token {
				let token = api_token.clone().expect("checked above");
				cfg.profile_mut(&effective.profile).token = Some(token);
				config::save_config(&config_path, &cfg)?;
				if !global.quiet {
					eprintln!("Token stored in profile '{}'.", effective.profile);
				}
			}

			if args.print_token {
				println!("{}", api_token.expect("checked above"));
				return Ok(());
			}

			if matches!(effective.output, OutputFormat::Table) {
				if let Some(user) = response.get("user") {
					print_kv(user);
				} else {
					print_kv(&response);
				}
				return Ok(());
			}

			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
	}
}

async fn run_org(global: &GlobalOpts, command: OrgCommand) -> Result<(), CliError> {
	let (_config_path, cfg) = load_config_store()?;
	let effective = resolve_effective_config(global, &cfg)?;

	let client = HttpClient::new(
		&effective.host,
		effective.token.clone(),
		effective.timeout,
		effective.retries,
		global.dry_run,
	)?;

	match command {
		OrgCommand::List(args) => {
			let mut response = client
				.request_json(Method::GET, "/api/v1/org", None, Default::default(), true)
				.await?;

			if args.details {
				let Some(orgs) = response.as_array() else {
					return Err(CliError::InvalidArgument("expected array response".to_string()));
				};

				let mut detailed = Vec::with_capacity(orgs.len());
				for org in orgs {
					let Some(id) = org.get("id").and_then(|v| v.as_str()) else {
						continue;
					};
					let detail = client
						.request_json(
							Method::GET,
							&format!("/api/v1/org/{id}"),
							None,
							Default::default(),
							true,
						)
						.await?;
					detailed.push(detail);
				}
				response = Value::Array(detailed);
			}

			if args.ids_only {
				let ids = response
					.as_array()
					.map(|arr| {
						arr.iter()
							.filter_map(|o| o.get("id").and_then(|v| v.as_str()).map(str::to_string))
							.collect::<Vec<_>>()
					})
					.unwrap_or_default();

				if matches!(effective.output, OutputFormat::Table) {
					for id in ids {
						println!("{id}");
					}
					return Ok(());
				}

				output::print_value(&Value::Array(ids.into_iter().map(Value::String).collect()), effective.output, global.no_color)?;
				return Ok(());
			}

			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
		OrgCommand::Get(args) => {
			let org_id = resolve_org_id(&client, &args.org).await?;
			let response = client
				.request_json(
					Method::GET,
					&format!("/api/v1/org/{org_id}"),
					None,
					Default::default(),
					true,
				)
				.await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
		OrgCommand::Users { command } => match command {
			crate::cli::OrgUsersCommand::List(args) => {
				let org_id = resolve_org_id(&client, &args.org).await?;
				let response = client
					.request_json(
						Method::GET,
						&format!("/api/v1/org/{org_id}/user"),
						None,
						Default::default(),
						true,
					)
					.await?;
				output::print_value(&response, effective.output, global.no_color)?;
				Ok(())
			}
		},
	}
}

fn load_config_store() -> Result<(PathBuf, Config), CliError> {
	let config_path = config::default_config_path()?;
	let cfg = config::load_config(&config_path)?;
	Ok((config_path, cfg))
}

fn read_stdin_trimmed() -> Result<String, CliError> {
	let mut input = String::new();
	io::stdin().read_to_string(&mut input)?;
	Ok(input.trim().to_string())
}

fn print_human_or_machine(
	value: &Value,
	format: OutputFormat,
	no_color: bool,
) -> Result<(), CliError> {
	if matches!(format, OutputFormat::Table) {
		print_kv(value);
		return Ok(());
	}
	output::print_value(value, format, no_color)
}

fn print_kv(value: &Value) {
	let Some(obj) = value.as_object() else {
		println!("{value}");
		return;
	};

	let mut keys: Vec<&String> = obj.keys().collect();
	keys.sort();
	for key in keys {
		let v = &obj[key];
		println!("{key}: {}", render_scalar(v));
	}
}

fn render_scalar(value: &Value) -> String {
	match value {
		Value::Null => String::new(),
		Value::Bool(v) => v.to_string(),
		Value::Number(v) => v.to_string(),
		Value::String(v) => v.clone(),
		_ => value.to_string(),
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
				"host" => p.host = Some(value.to_string()),
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

fn opt_string(value: Option<String>) -> Value {
	value.map(Value::String).unwrap_or(Value::Null)
}

fn redact_token(token: &str) -> String {
	const KEEP: usize = 4;
	if token.len() <= KEEP * 2 {
		return "REDACTED".to_string();
	}
	format!(
		"{}…{}",
		&token[..KEEP],
		&token[token.len() - KEEP..]
	)
}

async fn resolve_org_id(client: &HttpClient, org: &str) -> Result<String, CliError> {
	let org = org.trim();
	if org.is_empty() {
		return Err(CliError::InvalidArgument("org cannot be empty".to_string()));
	}

	let list = client
		.request_json(Method::GET, "/api/v1/org", None, Default::default(), true)
		.await?;

	let Some(orgs) = list.as_array() else {
		return Ok(org.to_string());
	};

	if orgs
		.iter()
		.any(|o| o.get("id").and_then(|v| v.as_str()) == Some(org))
	{
		return Ok(org.to_string());
	}

	let mut matches = Vec::new();
	for o in orgs {
		let id = o.get("id").and_then(|v| v.as_str());
		let name = o
			.get("orgName")
			.and_then(|v| v.as_str())
			.or_else(|| o.get("name").and_then(|v| v.as_str()));

		if let (Some(id), Some(name)) = (id, name) {
			if name.eq_ignore_ascii_case(org) {
				matches.push(id.to_string());
			}
		}
	}

	match matches.len() {
		0 => Ok(org.to_string()),
		1 => Ok(matches.remove(0)),
		_ => Err(CliError::InvalidArgument(format!(
			"org name '{org}' is ambiguous"
		))),
	}
}
