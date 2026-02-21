use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{GlobalOpts, NetworkCommand, OutputFormat};
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::http::HttpClient;
use crate::output;

use super::common::{load_config_store, print_human_or_machine};
use super::member;
use super::resolve::{extract_network_id, resolve_network_id, resolve_org_id};

pub(super) async fn run(global: &GlobalOpts, command: NetworkCommand) -> Result<(), CliError> {
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
			member::run_network_member(global, &effective, &client, command).await
		}
	}
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
				private_is = Some(matches!(
					v.trim().to_ascii_lowercase().as_str(),
					"true" | "1" | "yes"
				));
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
				if !name
					.to_ascii_lowercase()
					.contains(&needle.to_ascii_lowercase())
				{
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

