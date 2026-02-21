use std::io::{self, Read, Write};
use std::path::PathBuf;

use clap::CommandFactory;
use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{
	AuthCommand, Cli, Command, ConfigCommand, GlobalOpts, MemberCommand, NetworkCommand,
	NetworkMemberCommand, OrgCommand, OutputFormat, PlanetCommand, StatsCommand, ExportCommand,
	UserCommand,
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
		Command::Network { command } => run_network(&global, command).await,
		Command::Member { command } => run_member_alias(&global, command).await,
		Command::Stats { command } => run_stats(&global, command).await,
		Command::Planet { command } => run_planet(&global, command).await,
		Command::Export { command } => run_export(&global, command).await,
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

async fn run_network(global: &GlobalOpts, command: NetworkCommand) -> Result<(), CliError> {
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
		NetworkCommand::List(args) => {
			let org = args.org.or(effective.org.clone());
			let org_id = match org {
				Some(ref org) => Some(resolve_org_id(&client, org).await?),
				None => None,
			};

			let path = match org_id.as_deref() {
				Some(org_id) => format!("/api/v1/org/{org_id}/network"),
				None => "/api/v1/network".to_string(),
			};

			let mut response = client
				.request_json(Method::GET, &path, None, Default::default(), true)
				.await?;

			if let Some(filter) = args.filter.as_deref() {
				response = filter_network_list(response, filter)?;
			}

			if args.details {
				let Some(networks) = response.as_array() else {
					return Err(CliError::InvalidArgument("expected array response".to_string()));
				};

				let mut detailed = Vec::with_capacity(networks.len());
				for net in networks {
					let Some(id) = extract_network_id(net) else { continue };
					let detail_path = match org_id.as_deref() {
						Some(org_id) => format!("/api/v1/org/{org_id}/network/{id}"),
						None => format!("/api/v1/network/{id}"),
					};
					let detail = client
						.request_json(Method::GET, &detail_path, None, Default::default(), true)
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
							.filter_map(extract_network_id)
							.map(str::to_string)
							.collect::<Vec<_>>()
					})
					.unwrap_or_default();

				if matches!(effective.output, OutputFormat::Table) {
					for id in ids {
						println!("{id}");
					}
					return Ok(());
				}

				let value = Value::Array(ids.into_iter().map(Value::String).collect());
				output::print_value(&value, effective.output, global.no_color)?;
				return Ok(());
			}

			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
		NetworkCommand::Create(args) => {
			let org = args.org.or(effective.org.clone());
			let org_id = match org {
				Some(ref org) => Some(resolve_org_id(&client, org).await?),
				None => None,
			};

			let path = match org_id.as_deref() {
				Some(org_id) => format!("/api/v1/org/{org_id}/network"),
				None => "/api/v1/network".to_string(),
			};

			let body = args
				.name
				.map(|name| json!({ "name": name }))
				.unwrap_or_else(|| json!({}));

			let response = client
				.request_json(Method::POST, &path, Some(body), Default::default(), true)
				.await?;

			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
		NetworkCommand::Get(args) => {
			let org = args.org.or(effective.org.clone());
			let org_id = match org {
				Some(ref org) => Some(resolve_org_id(&client, org).await?),
				None => None,
			};

			let network_id = resolve_network_id(&client, org_id.as_deref(), &args.network).await?;
			let path = match org_id.as_deref() {
				Some(org_id) => format!("/api/v1/org/{org_id}/network/{network_id}"),
				None => format!("/api/v1/network/{network_id}"),
			};

			let response = client
				.request_json(Method::GET, &path, None, Default::default(), true)
				.await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
		NetworkCommand::Update(args) => {
			let org_id = resolve_org_id(&client, &args.org).await?;
			let network_id = resolve_network_id(&client, Some(&org_id), &args.network).await?;
			let path = format!("/api/v1/org/{org_id}/network/{network_id}");

			let body = if let Some(body) = args.body {
				serde_json::from_str::<Value>(&body)
					.map_err(|err| CliError::InvalidArgument(format!("invalid --body json: {err}")))?				
			} else if let Some(path) = args.body_file {
				let text = std::fs::read_to_string(&path)?;
				serde_json::from_str::<Value>(&text).map_err(|err| {
					CliError::InvalidArgument(format!("invalid --body-file json: {err}"))
				})?
			} else {
				build_network_update_body(&args)?
			};

			let response = client
				.request_json(Method::POST, &path, Some(body), Default::default(), true)
				.await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
		NetworkCommand::Member { command } => {
			run_network_member(global, &effective, &client, command).await
		}
	}
}

async fn run_member_alias(global: &GlobalOpts, command: MemberCommand) -> Result<(), CliError> {
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
		MemberCommand::List(args) => member_list(global, &effective, &client, args).await,
		MemberCommand::Get(args) => member_get(global, &effective, &client, args).await,
		MemberCommand::Update(args) => member_update(global, &effective, &client, args).await,
		MemberCommand::Authorize(args) => {
			member_set_authorized(
				global,
				&effective,
				&client,
				args.network,
				args.member,
				args.org,
				true,
			)
			.await
		}
		MemberCommand::Deauthorize(args) => {
			member_set_authorized(
				global,
				&effective,
				&client,
				args.network,
				args.member,
				args.org,
				false,
			)
			.await
		}
		MemberCommand::Delete(args) => member_delete(global, &effective, &client, args).await,
	}
}

async fn run_network_member(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	client: &HttpClient,
	command: NetworkMemberCommand,
) -> Result<(), CliError> {
	match command {
		NetworkMemberCommand::List(args) => member_list(global, effective, client, args).await,
		NetworkMemberCommand::Get(args) => member_get(global, effective, client, args).await,
		NetworkMemberCommand::Update(args) => member_update(global, effective, client, args).await,
		NetworkMemberCommand::Authorize(args) => {
			member_set_authorized(
				global,
				effective,
				client,
				args.network,
				args.member,
				args.org,
				true,
			)
			.await
		}
		NetworkMemberCommand::Deauthorize(args) => {
			member_set_authorized(
				global,
				effective,
				client,
				args.network,
				args.member,
				args.org,
				false,
			)
			.await
		}
		NetworkMemberCommand::Delete(args) => member_delete(global, effective, client, args).await,
	}
}

async fn member_list(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	client: &HttpClient,
	args: crate::cli::MemberListArgs,
) -> Result<(), CliError> {
	let org = args.org.or(effective.org.clone());
	let org_id = match org {
		Some(ref org) => Some(resolve_org_id(client, org).await?),
		None => None,
	};

	let network_id = resolve_network_id(client, org_id.as_deref(), &args.network).await?;
	let path = match org_id.as_deref() {
		Some(org_id) => format!("/api/v1/org/{org_id}/network/{network_id}/member"),
		None => format!("/api/v1/network/{network_id}/member"),
	};

	let mut response = client
		.request_json(Method::GET, &path, None, Default::default(), true)
		.await?;

	if args.authorized || args.unauthorized || args.name.is_some() || args.id.is_some() {
		let Some(items) = response.as_array() else {
			return Err(CliError::InvalidArgument("expected array response".to_string()));
		};

		let needle_name = args.name.as_deref().map(|s| s.to_ascii_lowercase());
		let needle_id = args.id.as_deref();

		let filtered: Vec<Value> = items
			.iter()
			.filter(|item| {
				if args.authorized {
					if item.get("authorized").and_then(|v| v.as_bool()) != Some(true) {
						return false;
					}
				}
				if args.unauthorized {
					if item.get("authorized").and_then(|v| v.as_bool()) != Some(false) {
						return false;
					}
				}
				if let Some(ref needle) = needle_name {
					let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
					if !name.to_ascii_lowercase().contains(needle) {
						return false;
					}
				}
				if let Some(needle) = needle_id {
					let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
					if id != needle {
						return false;
					}
				}
				true
			})
			.cloned()
			.collect();

		response = Value::Array(filtered);
	}

	output::print_value(&response, effective.output, global.no_color)?;
	Ok(())
}

async fn member_get(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	client: &HttpClient,
	args: crate::cli::MemberGetArgs,
) -> Result<(), CliError> {
	let org = args.org.or(effective.org.clone());
	let org_id = match org {
		Some(ref org) => Some(resolve_org_id(client, org).await?),
		None => None,
	};

	let network_id = resolve_network_id(client, org_id.as_deref(), &args.network).await?;

	let response = if let Some(org_id) = org_id.as_deref() {
		let path = format!("/api/v1/org/{org_id}/network/{network_id}/member/{}", args.member);
		client
			.request_json(Method::GET, &path, None, Default::default(), true)
			.await?
	} else {
		let path = format!("/api/v1/network/{network_id}/member");
		let list = client
			.request_json(Method::GET, &path, None, Default::default(), true)
			.await?;

		let Some(items) = list.as_array() else {
			return Err(CliError::InvalidArgument("expected array response".to_string()));
		};

		items
			.iter()
			.find(|item| item.get("id").and_then(|v| v.as_str()) == Some(args.member.as_str()))
			.cloned()
			.ok_or(CliError::HttpStatus {
				status: reqwest::StatusCode::NOT_FOUND,
				message: "member not found".to_string(),
				body: None,
			})?
	};

	print_human_or_machine(&response, effective.output, global.no_color)?;
	Ok(())
}

async fn member_update(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	client: &HttpClient,
	args: crate::cli::MemberUpdateArgs,
) -> Result<(), CliError> {
	let org = args.org.or(effective.org.clone());
	let org_id = match org {
		Some(ref org) => Some(resolve_org_id(client, org).await?),
		None => None,
	};

	let network_id = resolve_network_id(client, org_id.as_deref(), &args.network).await?;

	let body = if let Some(body) = args.body {
		serde_json::from_str::<Value>(&body)
			.map_err(|err| CliError::InvalidArgument(format!("invalid --body json: {err}")))?
	} else if let Some(path) = args.body_file {
		let text = std::fs::read_to_string(&path)?;
		serde_json::from_str::<Value>(&text)
			.map_err(|err| CliError::InvalidArgument(format!("invalid --body-file json: {err}")))?
	} else {
		let mut map = serde_json::Map::new();
		if let Some(name) = args.name {
			map.insert("name".to_string(), Value::String(name));
		}
		if org_id.is_none() {
			if let Some(description) = args.description {
				map.insert("description".to_string(), Value::String(description));
			}
		}
		if args.authorized {
			map.insert("authorized".to_string(), Value::Bool(true));
		} else if args.unauthorized {
			map.insert("authorized".to_string(), Value::Bool(false));
		}

		if map.is_empty() {
			return Err(CliError::InvalidArgument(
				"no update fields provided (use flags or --body/--body-file)".to_string(),
			));
		}
		Value::Object(map)
	};

	let path = match org_id.as_deref() {
		Some(org_id) => format!(
			"/api/v1/org/{org_id}/network/{network_id}/member/{}",
			args.member
		),
		None => format!("/api/v1/network/{network_id}/member/{}", args.member),
	};

	let response = client
		.request_json(Method::POST, &path, Some(body), Default::default(), true)
		.await?;

	print_human_or_machine(&response, effective.output, global.no_color)?;
	Ok(())
}

async fn member_set_authorized(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	client: &HttpClient,
	network: String,
	member: String,
	org: Option<String>,
	authorized: bool,
) -> Result<(), CliError> {
	let update = crate::cli::MemberUpdateArgs {
		network,
		member,
		org,
		name: None,
		description: None,
		authorized,
		unauthorized: !authorized,
		body: None,
		body_file: None,
	};
	member_update(global, effective, client, update).await
}

async fn member_delete(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	client: &HttpClient,
	args: crate::cli::MemberDeleteArgs,
) -> Result<(), CliError> {
	let org = args.org.or(effective.org.clone());
	let org_id = match org {
		Some(ref org) => Some(resolve_org_id(client, org).await?),
		None => None,
	};

	let network_id = resolve_network_id(client, org_id.as_deref(), &args.network).await?;

	let prompt = format!(
		"Delete (stash) member '{}' from network '{}'? ",
		args.member, network_id
	);
	if !confirm(global, &prompt)? {
		return Ok(());
	}

	let path = match org_id.as_deref() {
		Some(org_id) => format!(
			"/api/v1/org/{org_id}/network/{network_id}/member/{}",
			args.member
		),
		None => format!("/api/v1/network/{network_id}/member/{}", args.member),
	};

	let response = client
		.request_json(Method::DELETE, &path, None, Default::default(), true)
		.await?;
	print_human_or_machine(&response, effective.output, global.no_color)?;
	Ok(())
}

fn confirm(global: &GlobalOpts, prompt: &str) -> Result<bool, CliError> {
	if global.dry_run {
		return Ok(true);
	}
	if global.yes {
		return Ok(true);
	}
	if global.quiet {
		return Err(CliError::InvalidArgument(
			"refusing to prompt in --quiet mode (pass --yes)".to_string(),
		));
	}

	eprint!("{prompt}[y/N]: ");
	io::stderr().flush()?;

	let mut input = String::new();
	io::stdin().read_line(&mut input)?;
	let input = input.trim().to_ascii_lowercase();
	Ok(matches!(input.as_str(), "y" | "yes"))
}

async fn run_stats(global: &GlobalOpts, command: StatsCommand) -> Result<(), CliError> {
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
		StatsCommand::Get => {
			let response = client
				.request_json(Method::GET, "/api/v1/stats", None, Default::default(), true)
				.await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
	}
}

async fn run_planet(global: &GlobalOpts, command: PlanetCommand) -> Result<(), CliError> {
	let (_config_path, cfg) = load_config_store()?;
	let effective = resolve_effective_config(global, &cfg)?;

	match command {
		PlanetCommand::Download(args) => {
			if args.stdout && args.out.is_some() {
				return Err(CliError::InvalidArgument(
					"cannot combine --stdout with --out".to_string(),
				));
			}

			let out_path = if args.stdout {
				None
			} else {
				Some(args.out.unwrap_or_else(|| PathBuf::from("planet")))
			};

			if let Some(ref out_path) = out_path {
				if out_path.exists() && !args.force {
					return Err(CliError::InvalidArgument(format!(
						"output file already exists: {} (pass --force to overwrite)",
						out_path.display()
					)));
				}
			}

			let client = HttpClient::new(
				&effective.host,
				None,
				effective.timeout,
				effective.retries,
				global.dry_run,
			)?;

			let bytes = client
				.request_bytes(
					Method::GET,
					"/api/planet",
					None,
					Default::default(),
					false,
					None,
				)
				.await?;

			if let Some(out_path) = out_path {
				if let Some(parent) = out_path.parent() {
					std::fs::create_dir_all(parent)?;
				}
				std::fs::write(&out_path, &bytes)?;
				if !global.quiet {
					eprintln!("Wrote {} bytes to {}.", bytes.len(), out_path.display());
				}
				return Ok(());
			}

			io::stdout().write_all(&bytes)?;
			Ok(())
		}
	}
}

async fn run_export(global: &GlobalOpts, command: ExportCommand) -> Result<(), CliError> {
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
		ExportCommand::Hosts(args) => export_hosts(global, &effective, &client, args).await,
	}
}

async fn export_hosts(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	client: &HttpClient,
	args: crate::cli::ExportHostsArgs,
) -> Result<(), CliError> {
	if args.authorized_only && args.include_unauthorized {
		return Err(CliError::InvalidArgument(
			"cannot combine --authorized-only with --include-unauthorized".to_string(),
		));
	}

	let zone = args.zone.trim().trim_end_matches('.').to_string();
	if zone.is_empty() {
		return Err(CliError::InvalidArgument("--zone cannot be empty".to_string()));
	}

	let org = args.org.or(effective.org.clone());
	let org_id = match org {
		Some(ref org) => Some(resolve_org_id(client, org).await?),
		None => None,
	};

	let network_id = resolve_network_id(client, org_id.as_deref(), &args.network).await?;

	let network_get_path = match org_id.as_deref() {
		Some(org_id) => format!("/api/v1/org/{org_id}/network/{network_id}"),
		None => format!("/api/v1/network/{network_id}"),
	};

	let _network = client
		.request_json(Method::GET, &network_get_path, None, Default::default(), true)
		.await?;

	let member_list_path = match org_id.as_deref() {
		Some(org_id) => format!("/api/v1/org/{org_id}/network/{network_id}/member"),
		None => format!("/api/v1/network/{network_id}/member"),
	};

	let members = client
		.request_json(Method::GET, &member_list_path, None, Default::default(), true)
		.await?;

	let Some(items) = members.as_array() else {
		return Err(CliError::InvalidArgument("expected array response".to_string()));
	};

	let include_unauthorized = args.include_unauthorized;

	let mut records = Vec::new();
	for item in items {
		let authorized = item.get("authorized").and_then(|v| v.as_bool()).unwrap_or(false);
		if !include_unauthorized && !authorized {
			continue;
		}

		let member_id = item
			.get("id")
			.and_then(|v| v.as_str())
			.unwrap_or("")
			.to_string();

		let raw_name = item
			.get("name")
			.and_then(|v| v.as_str())
			.filter(|s| !s.trim().is_empty())
			.unwrap_or(member_id.as_str());

		let label = sanitize_hostname_label(raw_name);
		let hostname = format!("{label}.{zone}");

		let ips: Vec<String> = item
			.get("ipAssignments")
			.and_then(|v| v.as_array())
			.map(|arr| {
				arr.iter()
					.filter_map(|v| v.as_str().map(str::to_string))
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();

		for ip in ips {
			records.push(json!({
				"ip": ip,
				"hostname": hostname,
				"memberId": member_id,
				"name": raw_name,
				"authorized": authorized,
			}));
		}
	}

	match args.format {
		crate::cli::ExportHostsFormat::Json => {
			let value = Value::Array(records);
			write_export_output(&value, args.out.as_ref(), global)?;
		}
		crate::cli::ExportHostsFormat::Csv => {
			let mut out = String::new();
			out.push_str("ip,hostname,memberId,name,authorized\n");
			for r in &records {
				let ip = r.get("ip").and_then(|v| v.as_str()).unwrap_or("");
				let hostname = r.get("hostname").and_then(|v| v.as_str()).unwrap_or("");
				let member_id = r.get("memberId").and_then(|v| v.as_str()).unwrap_or("");
				let name = r.get("name").and_then(|v| v.as_str()).unwrap_or("");
				let authorized = r
					.get("authorized")
					.and_then(|v| v.as_bool())
					.unwrap_or(false);

				out.push_str(&format!(
					"{},{},{},{},{}\n",
					csv_escape(ip),
					csv_escape(hostname),
					csv_escape(member_id),
					csv_escape(name),
					authorized
				));
			}
			write_text_output(&out, args.out.as_ref(), global)?;
		}
		crate::cli::ExportHostsFormat::Hosts => {
			let mut out = String::new();
			for r in &records {
				let ip = r.get("ip").and_then(|v| v.as_str()).unwrap_or("");
				let hostname = r.get("hostname").and_then(|v| v.as_str()).unwrap_or("");
				out.push_str(&format!("{ip}\t{hostname}\n"));
			}
			write_text_output(&out, args.out.as_ref(), global)?;
		}
	}

	Ok(())
}

fn sanitize_hostname_label(value: &str) -> String {
	let mut out = String::with_capacity(value.len());
	for c in value.chars() {
		let c = c.to_ascii_lowercase();
		if matches!(c, 'a'..='z' | '0'..='9' | '-') {
			out.push(c);
		} else if c.is_whitespace() || matches!(c, '_' | '.') {
			out.push('-');
		}
	}

	let out = out.trim_matches('-').to_string();
	if out.is_empty() {
		"member".to_string()
	} else {
		out
	}
}

fn csv_escape(value: &str) -> String {
	if value.contains([',', '"', '\n', '\r']) {
		format!("\"{}\"", value.replace('"', "\"\""))
	} else {
		value.to_string()
	}
}

fn write_export_output(
	value: &Value,
	out: Option<&PathBuf>,
	global: &GlobalOpts,
) -> Result<(), CliError> {
	let json = serde_json::to_string_pretty(value)?;
	write_text_output(&json, out, global)
}

fn write_text_output(out: &str, path: Option<&PathBuf>, global: &GlobalOpts) -> Result<(), CliError> {
	if let Some(path) = path {
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent)?;
		}
		std::fs::write(path, out)?;
		if !global.quiet {
			eprintln!("Wrote {} bytes to {}.", out.as_bytes().len(), path.display());
		}
		return Ok(());
	}

	print!("{out}");
	Ok(())
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

async fn resolve_network_id(
	client: &HttpClient,
	org_id: Option<&str>,
	network: &str,
) -> Result<String, CliError> {
	let network = network.trim();
	if network.is_empty() {
		return Err(CliError::InvalidArgument("network cannot be empty".to_string()));
	}

	let list_path = match org_id {
		Some(org_id) => format!("/api/v1/org/{org_id}/network"),
		None => "/api/v1/network".to_string(),
	};

	let list = client
		.request_json(Method::GET, &list_path, None, Default::default(), true)
		.await?;

	let Some(networks) = list.as_array() else {
		return Ok(network.to_string());
	};

	if networks.iter().any(|n| extract_network_id(n) == Some(network)) {
		return Ok(network.to_string());
	}

	let mut matches = Vec::new();
	for n in networks {
		let id = extract_network_id(n);
		let name = n
			.get("name")
			.and_then(|v| v.as_str())
			.or_else(|| n.get("nwname").and_then(|v| v.as_str()));

		if let (Some(id), Some(name)) = (id, name) {
			if name.eq_ignore_ascii_case(network) {
				matches.push(id.to_string());
			}
		}
	}

	match matches.len() {
		0 => Ok(network.to_string()),
		1 => Ok(matches.remove(0)),
		_ => Err(CliError::InvalidArgument(format!(
			"network name '{network}' is ambiguous"
		))),
	}
}

fn extract_network_id(value: &Value) -> Option<&str> {
	value
		.get("id")
		.and_then(|v| v.as_str())
		.or_else(|| value.get("nwid").and_then(|v| v.as_str()))
}

fn filter_network_list(response: Value, expr: &str) -> Result<Value, CliError> {
	let Some(items) = response.as_array() else {
		return Ok(response);
	};

	let mut name_contains: Option<String> = None;
	let mut private_is: Option<bool> = None;

	for raw in expr.split(',').map(str::trim).filter(|s| !s.is_empty()) {
		if let Some((k, v)) = raw.split_once("~=") {
			if k.trim().eq_ignore_ascii_case("name") {
				name_contains = Some(v.trim().to_string());
			}
			continue;
		}
		if let Some((k, v)) = raw.split_once("==") {
			if k.trim().eq_ignore_ascii_case("private") {
				private_is = Some(matches!(v.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes"));
			}
			continue;
		}
	}

	let filtered: Vec<Value> = items
		.iter()
		.filter(|item| {
			if let Some(ref needle) = name_contains {
				let name = item
					.get("name")
					.and_then(|v| v.as_str())
					.or_else(|| item.get("nwname").and_then(|v| v.as_str()))
					.unwrap_or("");
				if !name.to_ascii_lowercase().contains(&needle.to_ascii_lowercase()) {
					return false;
				}
			}

			if let Some(expected) = private_is {
				let actual = item.get("private").and_then(|v| v.as_bool()).unwrap_or(false);
				if actual != expected {
					return false;
				}
			}

			true
		})
		.cloned()
		.collect();

	Ok(Value::Array(filtered))
}

fn build_network_update_body(args: &crate::cli::NetworkUpdateArgs) -> Result<Value, CliError> {
	let mut body = serde_json::Map::new();

	if let Some(name) = args.name.clone() {
		body.insert("name".to_string(), Value::String(name));
	}
	if let Some(description) = args.description.clone() {
		body.insert("description".to_string(), Value::String(description));
	}
	if let Some(mtu) = args.mtu.clone() {
		body.insert("mtu".to_string(), Value::String(mtu));
	}
	if args.private {
		body.insert("private".to_string(), Value::Bool(true));
	} else if args.public {
		body.insert("private".to_string(), Value::Bool(false));
	}

	if args.flow_rule.is_some() || args.flow_rule_file.is_some() {
		let rule = if let Some(rule) = args.flow_rule.clone() {
			rule
		} else if let Some(path) = args.flow_rule_file.as_ref() {
			std::fs::read_to_string(path)?
		} else {
			unreachable!()
		};
		body.insert("flowRule".to_string(), Value::String(rule));
	}

	if args.dns_domain.is_some() || !args.dns_server.is_empty() {
		let domain = args.dns_domain.clone().ok_or_else(|| {
			CliError::InvalidArgument("dns settings require --dns-domain".to_string())
		})?;
		let servers: Vec<Value> = args.dns_server.iter().cloned().map(Value::String).collect();
		body.insert(
			"dns".to_string(),
			json!({
				"domain": domain,
				"servers": servers,
			}),
		);
	}

	if body.is_empty() {
		return Err(CliError::InvalidArgument(
			"no update fields provided (use flags or --body/--body-file)".to_string(),
		));
	}

	Ok(Value::Object(body))
}
