use reqwest::Method;
use serde_json::Value;

use crate::cli::{GlobalOpts, OrgCommand, OrgRole, OutputFormat};
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::http::HttpClient;
use crate::output;

use super::common::{load_config_store, print_human_or_machine};
use super::resolve::resolve_org_id;
use super::trpc_client::{require_cookie_from_effective, TrpcClient};
use super::trpc_resolve::resolve_org_id as resolve_org_id_trpc;

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
			crate::cli::OrgUsersCommand::Add(args) => {
				let trpc = trpc_authed(global, &effective)?;
				let org_id = resolve_org_id_trpc(&trpc, &args.org).await?;

				let users = trpc
					.call(
						"org.getPlatformUsers",
						serde_json::json!({ "organizationId": &org_id }),
					)
					.await?;
				let Some(users) = users.as_array() else {
					return Err(CliError::InvalidArgument(
						"failed to list platform users".to_string(),
					));
				};

				let mut matches = Vec::new();
				for u in users {
					let email = u.get("email").and_then(|v| v.as_str()).unwrap_or("");
					if email.eq_ignore_ascii_case(&args.email) {
						matches.push(u.clone());
					}
				}

				let user = match matches.len() {
					0 => {
						return Err(CliError::InvalidArgument(format!(
							"user '{}' not found",
							args.email
						)));
					}
					1 => matches.remove(0),
					_ => {
						return Err(CliError::InvalidArgument(format!(
							"multiple users match '{}'",
							args.email
						)));
					}
				};

				let user_id = user
					.get("id")
					.and_then(|v| v.as_str())
					.ok_or_else(|| CliError::InvalidArgument("user missing id".to_string()))?
					.to_string();
				let user_name = user
					.get("name")
					.and_then(|v| v.as_str())
					.unwrap_or(&args.email)
					.to_string();

				let role = role_to_string(args.role);
				let response = trpc
					.call(
						"org.addUser",
						serde_json::json!({
							"organizationId": &org_id,
							"userId": user_id,
							"userName": user_name,
							"organizationRole": role,
						}),
					)
					.await?;

				print_human_or_machine(&response, effective.output, global.no_color)?;
				Ok(())
			}
			crate::cli::OrgUsersCommand::Role(args) => {
				let trpc = trpc_authed(global, &effective)?;
				let org_id = resolve_org_id_trpc(&trpc, &args.org).await?;

				let user_id = if args.user.contains('@') {
					let users = trpc
						.call("org.getOrgUsers", serde_json::json!({ "organizationId": &org_id }))
						.await?;
					let Some(users) = users.as_array() else {
						return Err(CliError::InvalidArgument(
							"failed to list org users".to_string(),
						));
					};

					let mut matches = Vec::new();
					for u in users {
						let email = u.get("email").and_then(|v| v.as_str()).unwrap_or("");
						if email.eq_ignore_ascii_case(&args.user) {
							matches.push(u.clone());
						}
					}

					let user = match matches.len() {
						0 => {
							return Err(CliError::InvalidArgument(format!(
								"user '{}' not found in org",
								args.user
							)));
						}
						1 => matches.remove(0),
						_ => {
							return Err(CliError::InvalidArgument(format!(
								"multiple org users match '{}'",
								args.user
							)));
						}
					};

					user.get("id")
						.and_then(|v| v.as_str())
						.ok_or_else(|| CliError::InvalidArgument("user missing id".to_string()))?
						.to_string()
				} else {
					args.user.clone()
				};

				let response = trpc
					.call(
						"org.changeUserRole",
						serde_json::json!({
							"organizationId": &org_id,
							"userId": user_id,
							"role": role_to_string(args.role),
						}),
					)
					.await?;

				print_human_or_machine(&response, effective.output, global.no_color)?;
				Ok(())
			}
		},
		OrgCommand::Invite { command } => {
			let trpc = trpc_authed(global, &effective)?;
			match command {
				crate::cli::OrgInviteCommand::Create(args) => {
					let org_id = resolve_org_id_trpc(&trpc, &args.org).await?;
					let response = trpc
						.call(
							"org.generateInviteLink",
							serde_json::json!({
								"organizationId": org_id,
								"role": role_to_string(args.role),
								"email": args.email,
							}),
						)
						.await?;
					print_human_or_machine(&response, effective.output, global.no_color)?;
					Ok(())
				}
				crate::cli::OrgInviteCommand::List(args) => {
					let org_id = resolve_org_id_trpc(&trpc, &args.org).await?;
					let response = trpc
						.call("org.getInvites", serde_json::json!({ "organizationId": org_id }))
						.await?;
					output::print_value(&response, effective.output, global.no_color)?;
					Ok(())
				}
				crate::cli::OrgInviteCommand::Delete(args) => {
					let org_id = resolve_org_id_trpc(&trpc, &args.org).await?;
					let response = trpc
						.call(
							"org.deleteInvite",
							serde_json::json!({
								"organizationId": org_id,
								"invitationId": args.invite,
							}),
						)
						.await?;
					print_human_or_machine(&response, effective.output, global.no_color)?;
					Ok(())
				}
				crate::cli::OrgInviteCommand::Send(args) => {
					let org_id = resolve_org_id_trpc(&trpc, &args.org).await?;
					let response = trpc
						.call(
							"org.inviteUserByMail",
							serde_json::json!({
								"organizationId": org_id,
								"role": role_to_string(args.role),
								"email": args.email,
							}),
						)
						.await?;
					print_human_or_machine(&response, effective.output, global.no_color)?;
					Ok(())
				}
			}
		}
		OrgCommand::Settings { command } => {
			let trpc = trpc_authed(global, &effective)?;
			match command {
				crate::cli::OrgSettingsCommand::Get(args) => {
					let org_id = resolve_org_id_trpc(&trpc, &args.org).await?;
					let response = trpc
						.call(
							"org.getOrganizationSettings",
							serde_json::json!({ "organizationId": org_id }),
						)
						.await?;
					print_human_or_machine(&response, effective.output, global.no_color)?;
					Ok(())
				}
				crate::cli::OrgSettingsCommand::Update(args) => {
					let org_id = resolve_org_id_trpc(&trpc, &args.org).await?;
					let rename = if args.rename_node_globally {
						Some(true)
					} else if args.no_rename_node_globally {
						Some(false)
					} else {
						None
					}
					.ok_or_else(|| {
						CliError::InvalidArgument(
							"no update fields provided (use --rename-node-globally or --no-rename-node-globally)"
								.to_string(),
						)
					})?;

					let response = trpc
						.call(
							"org.updateOrganizationSettings",
							serde_json::json!({
								"organizationId": org_id,
								"renameNodeGlobally": rename,
							}),
						)
						.await?;

					print_human_or_machine(&response, effective.output, global.no_color)?;
					Ok(())
				}
			}
		}
		OrgCommand::Webhooks { command } => {
			let trpc = trpc_authed(global, &effective)?;
			match command {
				crate::cli::OrgWebhooksCommand::List(args) => {
					let org_id = resolve_org_id_trpc(&trpc, &args.org).await?;
					let response = trpc
						.call("org.getOrgWebhooks", serde_json::json!({ "organizationId": org_id }))
						.await?;
					output::print_value(&response, effective.output, global.no_color)?;
					Ok(())
				}
				crate::cli::OrgWebhooksCommand::Add(args) => {
					if args.event.is_empty() {
						return Err(CliError::InvalidArgument(
							"webhook add requires at least one --event".to_string(),
						));
					}

					let org_id = resolve_org_id_trpc(&trpc, &args.org).await?;
					let response = trpc
						.call(
							"org.addOrgWebhooks",
							serde_json::json!({
								"organizationId": org_id,
								"webhookUrl": args.url,
								"webhookName": args.name,
								"hookType": args.event,
							}),
						)
						.await?;
					print_human_or_machine(&response, effective.output, global.no_color)?;
					Ok(())
				}
				crate::cli::OrgWebhooksCommand::Delete(args) => {
					let org_id = resolve_org_id_trpc(&trpc, &args.org).await?;
					let response = trpc
						.call(
							"org.deleteOrgWebhooks",
							serde_json::json!({
								"organizationId": org_id,
								"webhookId": args.webhook,
							}),
						)
						.await?;
					print_human_or_machine(&response, effective.output, global.no_color)?;
					Ok(())
				}
			}
		}
		OrgCommand::Logs(args) => {
			let trpc = trpc_authed(global, &effective)?;
			let org_id = resolve_org_id_trpc(&trpc, &args.org).await?;
			let response = trpc
				.call("org.getLogs", serde_json::json!({ "organizationId": org_id }))
				.await?;
			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
	}
}

fn role_to_string(role: OrgRole) -> &'static str {
	match role {
		OrgRole::ReadOnly => "READ_ONLY",
		OrgRole::User => "USER",
		OrgRole::Admin => "ADMIN",
	}
}

fn trpc_authed(global: &GlobalOpts, effective: &crate::context::EffectiveConfig) -> Result<TrpcClient, CliError> {
	let cookie = require_cookie_from_effective(effective)?;
	Ok(TrpcClient::new(
		&effective.host,
		effective.timeout,
		effective.retries,
		global.dry_run,
	)?
	.with_cookie(Some(cookie)))
}
