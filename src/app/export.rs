use std::path::PathBuf;

use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{ExportCommand, GlobalOpts};
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::http::{ClientUi, HttpClient};

use super::common::{load_config_store, write_text_output};
use super::resolve::{resolve_network_id, resolve_org_id};

pub(super) async fn run(global: &GlobalOpts, command: ExportCommand) -> Result<(), CliError> {
	let (_config_path, cfg) = load_config_store()?;
	let effective = resolve_effective_config(global, &cfg)?;

	let client = HttpClient::new(
		&effective.host,
		effective.token.clone(),
		effective.timeout,
		effective.retries,
		global.dry_run,
		ClientUi::new(global.quiet, global.no_color, Some(effective.profile.clone())),
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
	if value.contains([',', '\"', '\n', '\r']) {
		format!("\"{}\"", value.replace('\"', "\"\""))
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
