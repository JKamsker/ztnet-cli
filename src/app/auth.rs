use reqwest::Method;
use serde_json::json;

use crate::cli::{AuthCommand, GlobalOpts, OutputFormat};
use crate::config;
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::http::HttpClient;
use crate::output;

use super::common::{load_config_store, print_human_or_machine, read_stdin_trimmed, redact_token};

pub(super) async fn run(global: &GlobalOpts, command: AuthCommand) -> Result<(), CliError> {
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

