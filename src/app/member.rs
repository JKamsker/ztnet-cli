use reqwest::Method;
use serde_json::Value;

use crate::cli::{GlobalOpts, MemberCommand, NetworkMemberCommand, OutputFormat};
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::http::{ClientUi, HttpClient};
use crate::output;

use super::common::{confirm, load_config_store, print_human_or_machine};
use super::resolve::{resolve_network_id, resolve_org_id};
use super::trpc_client::{require_cookie_from_effective, TrpcClient};
use super::trpc_resolve::{resolve_network_org_id, resolve_personal_network_id};

pub(super) async fn run_alias(global: &GlobalOpts, command: MemberCommand) -> Result<(), CliError> {
	let (_config_path, cfg) = load_config_store()?;
	let effective = resolve_effective_config(global, &cfg)?;

	let client = HttpClient::new(
		&effective.host,
		effective.token.clone(),
		effective.timeout,
		effective.retries,
		global.dry_run,
		ClientUi::from_context(global, &effective),
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
		MemberCommand::Add(args) => member_add_trpc(global, &effective, args).await,
		MemberCommand::Tags(args) => member_tags_trpc(global, &effective, args).await,
		MemberCommand::Delete(args) => member_delete(global, &effective, &client, args).await,
	}
}

pub(super) async fn run_network_member(
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
		NetworkMemberCommand::Add(args) => member_add_trpc(global, effective, args).await,
		NetworkMemberCommand::Tags(args) => member_tags_trpc(global, effective, args).await,
	}
}

async fn member_add_trpc(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	args: crate::cli::MemberAddArgs,
) -> Result<(), CliError> {
	let trpc = trpc_authed(global, effective)?;
	let network_id = resolve_personal_network_id(&trpc, &args.network).await?;
	let details = trpc
		.query(
			"network.getNetworkById",
			serde_json::json!({ "nwid": network_id, "central": false }),
		)
		.await?;
	let org_id = resolve_network_org_id(&trpc, effective, args.org.as_deref(), &details).await?;

	let mut input = serde_json::Map::new();
	input.insert("nwid".to_string(), Value::String(network_id));
	input.insert("id".to_string(), Value::String(args.node_id));
	input.insert("central".to_string(), Value::Bool(false));
	if let Some(org_id) = org_id {
		input.insert("organizationId".to_string(), Value::String(org_id));
	}

	let response = trpc.call("networkMember.create", Value::Object(input)).await?;
	print_human_or_machine(&response, effective.output, global.no_color)?;
	Ok(())
}

async fn member_tags_trpc(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	args: crate::cli::MemberTagsArgs,
) -> Result<(), CliError> {
	let trpc = trpc_authed(global, effective)?;
	let network_id = resolve_personal_network_id(&trpc, &args.network).await?;
	let details = trpc
		.query(
			"network.getNetworkById",
			serde_json::json!({ "nwid": network_id, "central": false }),
		)
		.await?;
	let org_id = resolve_network_org_id(&trpc, effective, args.org.as_deref(), &details).await?;

	match args.command {
		crate::cli::MemberTagsCommand::List => {
			let member = trpc
				.query(
					"networkMember.getMemberById",
					serde_json::json!({ "id": args.member, "nwid": network_id, "central": false }),
				)
				.await?;

			let tags = member.get("tags").cloned().unwrap_or(Value::Null);

			if matches!(effective.output, OutputFormat::Table) && tags.is_null() {
				println!("(no tags)");
				return Ok(());
			}

			output::print_value(&tags, effective.output, global.no_color)?;
			Ok(())
		}
		crate::cli::MemberTagsCommand::Set(set) => {
			let tags = serde_json::from_str::<Value>(&set.tags).map_err(|err| {
				CliError::InvalidArgument(format!("invalid --tags json: {err}"))
			})?;

			let mut update = serde_json::Map::new();
			update.insert("tags".to_string(), tags);

			let mut input = serde_json::Map::new();
			input.insert("nwid".to_string(), Value::String(network_id));
			input.insert("memberId".to_string(), Value::String(args.member));
			input.insert("central".to_string(), Value::Bool(false));
			if let Some(org_id) = org_id {
				input.insert("organizationId".to_string(), Value::String(org_id));
			}
			input.insert("updateParams".to_string(), Value::Object(update));

			let response = trpc.call("networkMember.Tags", Value::Object(input)).await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
	}
}

fn trpc_authed(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
) -> Result<TrpcClient, CliError> {
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

	// Some deployments don't support a stable REST GET-by-id endpoint for members (400/405).
	// Prefer GET-by-id when it works, but fall back to list+filter for consistent behavior.
	let response = if let Some(org_id) = org_id.as_deref() {
		let path = format!("/api/v1/org/{org_id}/network/{network_id}/member/{}", args.member);
		match client
			.request_json(Method::GET, &path, None, Default::default(), true)
			.await
		{
			Ok(v) => v,
			Err(CliError::HttpStatus { status, .. })
				if status == reqwest::StatusCode::BAD_REQUEST
					|| status == reqwest::StatusCode::METHOD_NOT_ALLOWED =>
			{
				member_get_via_list(client, Some(org_id), &network_id, &args.member).await?
			}
			Err(err) => return Err(err),
		}
	} else {
		member_get_via_list(client, None, &network_id, &args.member).await?
	};

	print_human_or_machine(&response, effective.output, global.no_color)?;
	Ok(())
}

async fn member_get_via_list(
	client: &HttpClient,
	org_id: Option<&str>,
	network_id: &str,
	member_id: &str,
) -> Result<Value, CliError> {
	let path = match org_id {
		Some(org_id) => format!("/api/v1/org/{org_id}/network/{network_id}/member"),
		None => format!("/api/v1/network/{network_id}/member"),
	};

	let list = client
		.request_json(Method::GET, &path, None, Default::default(), true)
		.await?;

	let Some(items) = list.as_array() else {
		return Err(CliError::InvalidArgument("expected array response".to_string()));
	};

	items
		.iter()
		.find(|item| item.get("id").and_then(|v| v.as_str()) == Some(member_id))
		.cloned()
		.ok_or(CliError::HttpStatus {
			status: reqwest::StatusCode::NOT_FOUND,
			message: "member not found".to_string(),
			body: None,
		})
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
