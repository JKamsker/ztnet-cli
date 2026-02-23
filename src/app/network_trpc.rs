use serde_json::{json, Value};

use crate::cli::{
	GlobalOpts, NetworkDeleteArgs, NetworkDnsArgs, NetworkFlowRulesArgs, NetworkFlowRulesCommand,
	NetworkIpPoolArgs, NetworkIpPoolCommand, NetworkIpv6Args, NetworkMulticastArgs,
	NetworkRoutesArgs, NetworkRoutesCommand, OutputFormat,
};
use crate::context::EffectiveConfig;
use crate::error::CliError;
use crate::http::ClientUi;
use crate::output;

use super::common::confirm;
use super::trpc_client::{require_cookie_from_effective, TrpcClient};
use super::trpc_resolve::{resolve_network_org_id, resolve_personal_network_id};

pub(super) async fn delete(
	global: &GlobalOpts,
	effective: &EffectiveConfig,
	args: NetworkDeleteArgs,
) -> Result<(), CliError> {
	let trpc = trpc_authed(global, effective)?;
	let network_id = resolve_personal_network_id(&trpc, &args.network).await?;
	let details = get_network_details(&trpc, &network_id).await?;

	let name = details
		.get("network")
		.and_then(|n| n.get("name"))
		.and_then(|v| v.as_str())
		.unwrap_or(&network_id);

	let prompt = format!("Delete network '{name}' ({network_id})? ");
	if !confirm(global, &prompt)? {
		return Ok(());
	}

	let org_id = resolve_network_org_id(&trpc, effective, args.org.as_deref(), &details).await?;

	let mut input = serde_json::Map::new();
	input.insert("nwid".to_string(), Value::String(network_id));
	input.insert("central".to_string(), Value::Bool(false));
	if let Some(org_id) = org_id {
		input.insert("organizationId".to_string(), Value::String(org_id));
	}

	let response = trpc.call("network.deleteNetwork", Value::Object(input)).await?;

	if matches!(effective.output, OutputFormat::Table) {
		println!("OK");
		return Ok(());
	}

	output::print_value(&response, effective.output, global.no_color)?;
	Ok(())
}

pub(super) async fn routes(
	global: &GlobalOpts,
	effective: &EffectiveConfig,
	args: NetworkRoutesArgs,
) -> Result<(), CliError> {
	let trpc = trpc_authed(global, effective)?;
	let network_id = resolve_personal_network_id(&trpc, &args.network).await?;
	let details = get_network_details(&trpc, &network_id).await?;
	let org_id = resolve_network_org_id(&trpc, effective, args.org.as_deref(), &details).await?;

	let mut routes = extract_network_routes(&details)?;

	match args.command {
		NetworkRoutesCommand::List => {
			output::print_value(&Value::Array(routes), effective.output, global.no_color)?;
			Ok(())
		}
		NetworkRoutesCommand::Add(add) => {
			let destination = add.destination.trim().to_string();
			if destination.is_empty() {
				return Err(CliError::InvalidArgument(
					"--destination cannot be empty".to_string(),
				));
			}

			if routes.iter().any(|r| {
				r.get("target").and_then(|v| v.as_str()) == Some(destination.as_str())
			}) {
				return Err(CliError::InvalidArgument(format!(
					"route '{destination}' already exists"
				)));
			}

			let via = match add.via.as_deref().map(str::trim) {
				Some("") | None => Value::Null,
				Some("lan") => Value::Null,
				Some(v) => Value::String(v.to_string()),
			};

			routes.push(json!({ "target": destination, "via": via }));

			let response = trpc
				.call("network.managedRoutes", managed_routes_input(network_id, org_id, routes))
				.await?;

			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
		NetworkRoutesCommand::Remove(remove) => {
			let destination = remove.destination.trim().to_string();
			let before = routes.len();
			routes.retain(|r| {
				r.get("target").and_then(|v| v.as_str()) != Some(destination.as_str())
			});

			if routes.len() == before {
				return Err(CliError::InvalidArgument(format!(
					"route '{destination}' not found"
				)));
			}

			let response = trpc
				.call("network.managedRoutes", managed_routes_input(network_id, org_id, routes))
				.await?;

			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
	}
}

pub(super) async fn ip_pool(
	global: &GlobalOpts,
	effective: &EffectiveConfig,
	args: NetworkIpPoolArgs,
) -> Result<(), CliError> {
	let trpc = trpc_authed(global, effective)?;
	let network_id = resolve_personal_network_id(&trpc, &args.network).await?;
	let details = get_network_details(&trpc, &network_id).await?;
	let org_id = resolve_network_org_id(&trpc, effective, args.org.as_deref(), &details).await?;

	match args.command {
		NetworkIpPoolCommand::List => {
			let pools = details
				.get("network")
				.and_then(|n| n.get("ipAssignmentPools"))
				.cloned()
				.unwrap_or(Value::Array(Vec::new()));

			output::print_value(&pools, effective.output, global.no_color)?;
			Ok(())
		}
		NetworkIpPoolCommand::Add(change) => {
			let (start, end) = pool_range(&change)?;
			let mut pools = extract_ip_pools(&details)?;

			if pools.iter().any(|p| pool_matches(p, &start, &end)) {
				return Err(CliError::InvalidArgument("pool already exists".to_string()));
			}

			pools.push(json!({ "ipRangeStart": start, "ipRangeEnd": end }));

			let response = trpc
				.call(
					"network.advancedIpAssignment",
					advanced_ip_assignment_input(network_id, org_id, pools),
				)
				.await?;

			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
		NetworkIpPoolCommand::Remove(change) => {
			let (start, end) = pool_range(&change)?;
			let mut pools = extract_ip_pools(&details)?;
			let before = pools.len();
			pools.retain(|p| !pool_matches(p, &start, &end));

			if pools.len() == before {
				return Err(CliError::InvalidArgument("pool not found".to_string()));
			}

			let response = trpc
				.call(
					"network.advancedIpAssignment",
					advanced_ip_assignment_input(network_id, org_id, pools),
				)
				.await?;

			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
	}
}

pub(super) async fn dns(
	global: &GlobalOpts,
	effective: &EffectiveConfig,
	args: NetworkDnsArgs,
) -> Result<(), CliError> {
	let trpc = trpc_authed(global, effective)?;
	let network_id = resolve_personal_network_id(&trpc, &args.network).await?;
	let details = get_network_details(&trpc, &network_id).await?;
	let org_id = resolve_network_org_id(&trpc, effective, args.org.as_deref(), &details).await?;

	let update_params = if args.clear {
		json!({ "clearDns": true })
	} else {
		let domain = args.domain.ok_or_else(|| {
			CliError::InvalidArgument("dns update requires --domain (or --clear)".to_string())
		})?;
		if args.servers.is_empty() {
			return Err(CliError::InvalidArgument(
				"dns update requires --servers (comma-separated)".to_string(),
			));
		}
		json!({ "dns": { "domain": domain, "servers": args.servers } })
	};

	let response = trpc
		.call("network.dns", dns_input(network_id, org_id, update_params))
		.await?;

	output::print_value(&response, effective.output, global.no_color)?;
	Ok(())
}

pub(super) async fn ipv6(
	global: &GlobalOpts,
	effective: &EffectiveConfig,
	args: NetworkIpv6Args,
) -> Result<(), CliError> {
	let trpc = trpc_authed(global, effective)?;
	let network_id = resolve_personal_network_id(&trpc, &args.network).await?;
	let details = get_network_details(&trpc, &network_id).await?;
	let org_id = resolve_network_org_id(&trpc, effective, args.org.as_deref(), &details).await?;

	let mut v6 = serde_json::Map::new();
	if args.sixplane {
		v6.insert("6plane".to_string(), Value::Bool(true));
	} else if args.no_6plane {
		v6.insert("6plane".to_string(), Value::Bool(false));
	}
	if args.rfc4193 {
		v6.insert("rfc4193".to_string(), Value::Bool(true));
	} else if args.no_rfc4193 {
		v6.insert("rfc4193".to_string(), Value::Bool(false));
	}
	if args.zt {
		v6.insert("zt".to_string(), Value::Bool(true));
	} else if args.no_zt {
		v6.insert("zt".to_string(), Value::Bool(false));
	}

	if v6.is_empty() {
		return Err(CliError::InvalidArgument(
			"no ipv6 options provided (use --6plane/--no-6plane, --rfc4193/--no-rfc4193, --zt/--no-zt)".to_string(),
		));
	}

	let response = trpc
		.call("network.ipv6", ipv6_input(network_id, org_id, v6))
		.await?;

	output::print_value(&response, effective.output, global.no_color)?;
	Ok(())
}

pub(super) async fn multicast(
	global: &GlobalOpts,
	effective: &EffectiveConfig,
	args: NetworkMulticastArgs,
) -> Result<(), CliError> {
	let trpc = trpc_authed(global, effective)?;
	let network_id = resolve_personal_network_id(&trpc, &args.network).await?;
	let details = get_network_details(&trpc, &network_id).await?;
	let org_id = resolve_network_org_id(&trpc, effective, args.org.as_deref(), &details).await?;

	let mut update = serde_json::Map::new();
	if let Some(limit) = args.limit {
		update.insert("multicastLimit".to_string(), Value::Number(limit.into()));
	}
	if args.enable {
		update.insert("enableBroadcast".to_string(), Value::Bool(true));
	} else if args.disable {
		update.insert("enableBroadcast".to_string(), Value::Bool(false));
	}

	if update.is_empty() {
		return Err(CliError::InvalidArgument(
			"no multicast options provided (use --limit and/or --enable/--disable)".to_string(),
		));
	}

	let response = trpc
		.call("network.multiCast", multicast_input(network_id, org_id, update))
		.await?;

	output::print_value(&response, effective.output, global.no_color)?;
	Ok(())
}

pub(super) async fn flow_rules(
	global: &GlobalOpts,
	effective: &EffectiveConfig,
	args: NetworkFlowRulesArgs,
) -> Result<(), CliError> {
	let trpc = trpc_authed(global, effective)?;
	let network_id = resolve_personal_network_id(&trpc, &args.network).await?;

	match args.command {
		NetworkFlowRulesCommand::Get(get) => {
			let response = trpc
				.query(
					"network.getFlowRule",
					json!({ "nwid": network_id, "central": false, "reset": get.reset }),
				)
				.await?;

			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
	}
}

fn trpc_authed(global: &GlobalOpts, effective: &EffectiveConfig) -> Result<TrpcClient, CliError> {
	let cookie = require_cookie_from_effective(effective)?;
	Ok(TrpcClient::new(
		&effective.host,
		effective.timeout,
		effective.retries,
		global.dry_run,
		ClientUi::from_context(global, effective),
	)?
	.with_cookie(Some(cookie)))
}

async fn get_network_details(trpc: &TrpcClient, nwid: &str) -> Result<Value, CliError> {
	trpc.query("network.getNetworkById", json!({ "nwid": nwid, "central": false }))
		.await
}

fn extract_network_routes(details: &Value) -> Result<Vec<Value>, CliError> {
	let routes = details
		.get("network")
		.and_then(|n| n.get("routes"))
		.and_then(|v| v.as_array())
		.cloned()
		.unwrap_or_default();

	let normalized = routes
		.into_iter()
		.filter_map(|r| {
			let target = r.get("target").and_then(|v| v.as_str())?;
			let via = r.get("via").cloned().unwrap_or(Value::Null);
			Some(json!({ "target": target, "via": via }))
		})
		.collect();

	Ok(normalized)
}

fn extract_ip_pools(details: &Value) -> Result<Vec<Value>, CliError> {
	let pools = details
		.get("network")
		.and_then(|n| n.get("ipAssignmentPools"))
		.and_then(|v| v.as_array())
		.cloned()
		.unwrap_or_default();

	let normalized = pools
		.into_iter()
		.filter_map(|p| {
			let start = p.get("ipRangeStart").and_then(|v| v.as_str())?;
			let end = p.get("ipRangeEnd").and_then(|v| v.as_str())?;
			Some(json!({ "ipRangeStart": start, "ipRangeEnd": end }))
		})
		.collect();

	Ok(normalized)
}

fn pool_matches(pool: &Value, start: &str, end: &str) -> bool {
	pool.get("ipRangeStart").and_then(|v| v.as_str()) == Some(start)
		&& pool.get("ipRangeEnd").and_then(|v| v.as_str()) == Some(end)
}

fn pool_range(args: &crate::cli::NetworkIpPoolChangeArgs) -> Result<(String, String), CliError> {
	if let Some(cidr) = args.cidr.as_deref() {
		return cidr_to_ipv4_range(cidr);
	}

	let start = args
		.start
		.as_deref()
		.ok_or_else(|| CliError::InvalidArgument("missing --start".to_string()))?
		.trim();
	let end = args
		.end
		.as_deref()
		.ok_or_else(|| CliError::InvalidArgument("missing --end".to_string()))?
		.trim();

	if start.is_empty() || end.is_empty() {
		return Err(CliError::InvalidArgument(
			"--start/--end cannot be empty".to_string(),
		));
	}

	Ok((start.to_string(), end.to_string()))
}

fn cidr_to_ipv4_range(cidr: &str) -> Result<(String, String), CliError> {
	let (ip, prefix) = cidr
		.trim()
		.split_once('/')
		.ok_or_else(|| CliError::InvalidArgument("invalid CIDR".to_string()))?;

	let ip = ip.trim().parse::<std::net::Ipv4Addr>().map_err(|_| {
		CliError::InvalidArgument("CIDR must be a valid IPv4 address".to_string())
	})?;

	let prefix = prefix.trim().parse::<u32>().map_err(|_| {
		CliError::InvalidArgument("CIDR prefix must be a number".to_string())
	})?;
	if prefix > 32 {
		return Err(CliError::InvalidArgument(
			"CIDR prefix must be <= 32".to_string(),
		));
	}

	let ip_u32 = u32::from(ip);
	let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
	let network = ip_u32 & mask;
	let broadcast = network | !mask;

	let (start, end) = match prefix {
		32 => (network, network),
		31 => (network, broadcast),
		_ => (network + 1, broadcast - 1),
	};

	Ok((std::net::Ipv4Addr::from(start).to_string(), std::net::Ipv4Addr::from(end).to_string()))
}

fn managed_routes_input(nwid: String, org_id: Option<String>, routes: Vec<Value>) -> Value {
	let mut input = serde_json::Map::new();
	input.insert("nwid".to_string(), Value::String(nwid));
	input.insert("central".to_string(), Value::Bool(false));
	if let Some(org_id) = org_id {
		input.insert("organizationId".to_string(), Value::String(org_id));
	}
	input.insert("updateParams".to_string(), json!({ "routes": routes }));
	Value::Object(input)
}

fn advanced_ip_assignment_input(nwid: String, org_id: Option<String>, pools: Vec<Value>) -> Value {
	let mut input = serde_json::Map::new();
	input.insert("nwid".to_string(), Value::String(nwid));
	input.insert("central".to_string(), Value::Bool(false));
	if let Some(org_id) = org_id {
		input.insert("organizationId".to_string(), Value::String(org_id));
	}
	input.insert("updateParams".to_string(), json!({ "ipAssignmentPools": pools }));
	Value::Object(input)
}

fn dns_input(nwid: String, org_id: Option<String>, update_params: Value) -> Value {
	let mut input = serde_json::Map::new();
	input.insert("nwid".to_string(), Value::String(nwid));
	input.insert("central".to_string(), Value::Bool(false));
	if let Some(org_id) = org_id {
		input.insert("organizationId".to_string(), Value::String(org_id));
	}
	input.insert("updateParams".to_string(), update_params);
	Value::Object(input)
}

fn ipv6_input(nwid: String, org_id: Option<String>, v6_assign_mode: serde_json::Map<String, Value>) -> Value {
	let mut input = serde_json::Map::new();
	input.insert("nwid".to_string(), Value::String(nwid));
	input.insert("central".to_string(), Value::Bool(false));
	if let Some(org_id) = org_id {
		input.insert("organizationId".to_string(), Value::String(org_id));
	}
	input.insert("v6AssignMode".to_string(), Value::Object(v6_assign_mode));
	Value::Object(input)
}

fn multicast_input(nwid: String, org_id: Option<String>, update_params: serde_json::Map<String, Value>) -> Value {
	let mut input = serde_json::Map::new();
	input.insert("nwid".to_string(), Value::String(nwid));
	input.insert("central".to_string(), Value::Bool(false));
	if let Some(org_id) = org_id {
		input.insert("organizationId".to_string(), Value::String(org_id));
	}
	input.insert("updateParams".to_string(), Value::Object(update_params));
	Value::Object(input)
}
