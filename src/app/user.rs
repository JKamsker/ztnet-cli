use reqwest::Method;
use serde_json::Value;

use crate::cli::{GlobalOpts, OutputFormat, UserCommand};
use crate::config;
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::http::HttpClient;
use crate::output;

use super::common::{load_config_store, print_kv};

pub(super) async fn run(global: &GlobalOpts, command: UserCommand) -> Result<(), CliError> {
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

