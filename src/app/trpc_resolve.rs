use serde_json::Value;

use crate::context::EffectiveConfig;
use crate::error::CliError;

use super::trpc_client::TrpcClient;

pub(super) async fn resolve_org_id(trpc: &TrpcClient, org: &str) -> Result<String, CliError> {
	let org = org.trim();
	if org.is_empty() {
		return Err(CliError::InvalidArgument("org cannot be empty".to_string()));
	}

	let value = trpc.query("org.getOrgIdbyUserid", Value::Null).await?;
	let Some(items) = value.as_array() else {
		return Ok(org.to_string());
	};

	if items
		.iter()
		.any(|o| o.get("id").and_then(|v| v.as_str()) == Some(org))
	{
		return Ok(org.to_string());
	}

	let mut matches = Vec::new();
	for item in items {
		let id = item.get("id").and_then(|v| v.as_str());
		let name = item.get("orgName").and_then(|v| v.as_str());
		if let (Some(id), Some(name)) = (id, name) {
			if name.eq_ignore_ascii_case(org) {
				matches.push(id.to_string());
			}
		}
	}

	match matches.len() {
		0 => Err(CliError::InvalidArgument(format!(
			"org '{org}' not found (pass org id or exact orgName)"
		))),
		1 => Ok(matches.remove(0)),
		_ => Err(CliError::InvalidArgument(format!(
			"org name '{org}' is ambiguous"
		))),
	}
}

pub(super) async fn resolve_personal_network_id(
	trpc: &TrpcClient,
	network: &str,
) -> Result<String, CliError> {
	let network = network.trim();
	if network.is_empty() {
		return Err(CliError::InvalidArgument("network cannot be empty".to_string()));
	}

	if is_network_id(network) {
		return Ok(network.to_string());
	}

	let input = Value::Object(Default::default());
	let value = trpc.query("network.getUserNetworks", input).await?;
	let Some(items) = value.as_array() else {
		return Err(CliError::InvalidArgument(
			"failed to list networks for name resolution".to_string(),
		));
	};

	let mut matches = Vec::new();
	for item in items {
		let id = item.get("nwid").and_then(|v| v.as_str());
		let name = item.get("name").and_then(|v| v.as_str());
		if let Some(id) = id {
			if id == network {
				return Ok(id.to_string());
			}
		}
		if let (Some(id), Some(name)) = (id, name) {
			if name.eq_ignore_ascii_case(network) {
				matches.push(id.to_string());
			}
		}
	}

	match matches.len() {
		0 => Err(CliError::InvalidArgument(format!(
			"network '{network}' not found (tRPC commands require a network id; name resolution works for personal networks only)"
		))),
		1 => Ok(matches.remove(0)),
		_ => Err(CliError::InvalidArgument(format!(
			"network name '{network}' is ambiguous"
		))),
	}
}

pub(super) async fn resolve_network_org_id(
	trpc: &TrpcClient,
	_effective: &EffectiveConfig,
	cli_org: Option<&str>,
	network_details: &Value,
) -> Result<Option<String>, CliError> {
	let inferred = network_details
		.get("network")
		.and_then(|n| n.get("organizationId"))
		.and_then(|v| v.as_str())
		.map(str::to_string);

	let explicit = match cli_org {
		Some(org) => Some(resolve_org_id(trpc, org).await?),
		None => None,
	};

	match inferred {
		Some(inferred) => {
			if let Some(explicit) = explicit {
				if explicit != inferred {
					return Err(CliError::InvalidArgument(
						"network belongs to a different org than --org".to_string(),
					));
				}
			}
			Ok(Some(inferred))
		}
		None => Ok(explicit),
	}
}

fn is_network_id(value: &str) -> bool {
	value.len() == 16 && value.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
	use super::is_network_id;

	#[test]
	fn is_network_id_accepts_16_hex_chars() {
		assert!(is_network_id("9ad07d01093a69e3"));
	}

	#[test]
	fn is_network_id_rejects_10_hex_chars() {
		assert!(!is_network_id("b621d170ad"));
	}

	#[test]
	fn is_network_id_rejects_non_hex() {
		assert!(!is_network_id("9ad07d01093a69eg"));
	}
}
