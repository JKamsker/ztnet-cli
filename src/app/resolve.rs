use reqwest::Method;
use serde_json::Value;

use crate::error::CliError;
use crate::http::HttpClient;

pub(super) async fn resolve_org_id(client: &HttpClient, org: &str) -> Result<String, CliError> {
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

pub(super) async fn resolve_network_id(
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

pub(super) fn extract_network_id(value: &Value) -> Option<&str> {
	value
		.get("id")
		.and_then(|v| v.as_str())
		.or_else(|| value.get("nwid").and_then(|v| v.as_str()))
}

