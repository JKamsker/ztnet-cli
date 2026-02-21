use reqwest::Method;
use serde_json::Value;

use crate::cli::{GlobalOpts, OrgCommand, OutputFormat};
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::http::HttpClient;
use crate::output;

use super::common::{load_config_store, print_human_or_machine};
use super::resolve::resolve_org_id;

pub(super) async fn run(global: &GlobalOpts, command: OrgCommand) -> Result<(), CliError> {
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

				let value = Value::Array(ids.into_iter().map(Value::String).collect());
				output::print_value(&value, effective.output, global.no_color)?;
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

