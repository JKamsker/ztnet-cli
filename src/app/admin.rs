use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde_json::{json, Value};

use crate::cli::{
	AdminBackupCommand, AdminCommand, AdminInvitesCommand, AdminMailCommand,
	AdminMailTemplatesCommand, AdminSettingsCommand, AdminUsersCommand, GlobalOpts,
	MailTemplateKeyArg, OutputFormat, UserRole,
};
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::output;

use super::common::{confirm, load_config_store, print_human_or_machine};
use super::trpc_client::{require_cookie_from_effective, TrpcClient};

pub(super) async fn run(global: &GlobalOpts, command: AdminCommand) -> Result<(), CliError> {
	let (_config_path, cfg) = load_config_store()?;
	let effective = resolve_effective_config(global, &cfg)?;

	let trpc = trpc_authed(global, &effective)?;

	match command {
		AdminCommand::Users { command } => users(global, &effective, &trpc, command).await,
		AdminCommand::Backup { command } => backup(global, &effective, &trpc, command).await,
		AdminCommand::Mail { command } => mail(global, &effective, &trpc, command).await,
		AdminCommand::Settings { command } => settings(global, &effective, &trpc, command).await,
		AdminCommand::Invites { command } => invites(global, &effective, &trpc, command).await,
	}
}

async fn users(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	trpc: &TrpcClient,
	command: AdminUsersCommand,
) -> Result<(), CliError> {
	match command {
		AdminUsersCommand::List(args) => {
			let response = trpc
				.call("admin.getUsers", json!({ "isAdmin": args.admins }))
				.await?;
			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
		AdminUsersCommand::Get(args) => {
			let response = trpc
				.call("admin.getUser", json!({ "userId": args.user }))
				.await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
		AdminUsersCommand::Delete(args) => {
			let prompt = format!("Delete user '{}' ? ", args.user);
			if !confirm(global, &prompt)? {
				return Ok(());
			}
			let response = trpc
				.call("admin.deleteUser", json!({ "id": args.user }))
				.await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
		AdminUsersCommand::Update(args) => {
			if args.role.is_none() && !args.active && !args.inactive {
				return Err(CliError::InvalidArgument(
					"no update fields provided (use --role and/or --active/--inactive)".to_string(),
				));
			}

			let mut result = serde_json::Map::new();

			if let Some(role) = args.role {
				let response = trpc
					.call(
						"admin.changeRole",
						json!({ "id": &args.user, "role": user_role_to_string(role) }),
					)
					.await?;
				result.insert("role".to_string(), response);
			}

			if args.active || args.inactive {
				let is_active = args.active;
				let response = trpc
					.call(
						"admin.updateUser",
						json!({ "id": &args.user, "params": { "isActive": is_active } }),
					)
					.await?;
				result.insert("status".to_string(), response);
			}

			if matches!(effective.output, OutputFormat::Table) && result.is_empty() {
				println!("OK");
				return Ok(());
			}

			print_human_or_machine(&Value::Object(result), effective.output, global.no_color)?;
			Ok(())
		}
	}
}

async fn backup(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	trpc: &TrpcClient,
	command: AdminBackupCommand,
) -> Result<(), CliError> {
	match command {
		AdminBackupCommand::List => {
			let response = trpc.call("admin.listBackups", Value::Null).await?;
			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
		AdminBackupCommand::Create(args) => {
			let mut input = serde_json::Map::new();
			input.insert("includeDatabase".to_string(), Value::Bool(!args.no_database));
			input.insert("includeZerotier".to_string(), Value::Bool(!args.no_zerotier));
			if let Some(name) = args.name {
				input.insert("backupName".to_string(), Value::String(name));
			}

			let response = trpc.call("admin.createBackup", Value::Object(input)).await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
		AdminBackupCommand::Download(args) => {
			let response = trpc
				.call("admin.downloadBackup", json!({ "fileName": args.backup }))
				.await?;

			let data = response
				.get("data")
				.and_then(|v| v.as_str())
				.ok_or_else(|| CliError::InvalidArgument("backup download returned no data".to_string()))?;

			let bytes = base64::engine::general_purpose::STANDARD
				.decode(data)
				.map_err(|err| CliError::InvalidArgument(format!("invalid base64: {err}")))?;

			if let Some(parent) = args.out.parent() {
				std::fs::create_dir_all(parent)?;
			}
			std::fs::write(&args.out, bytes)?;

			if !global.quiet {
				eprintln!("Wrote backup to {}.", args.out.display());
			}

			if matches!(effective.output, OutputFormat::Table) {
				return Ok(());
			}

			let out = json!({ "out": args.out.to_string_lossy() });
			output::print_value(&out, effective.output, global.no_color)?;
			Ok(())
		}
		AdminBackupCommand::Restore(args) => {
			let prompt = format!("Restore backup '{}' ? ", args.backup);
			if !confirm(global, &prompt)? {
				return Ok(());
			}

			let response = trpc
				.call(
					"admin.restoreBackup",
					json!({
						"fileName": args.backup,
						"restoreDatabase": !args.no_database,
						"restoreZerotier": !args.no_zerotier,
					}),
				)
				.await?;

			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
		AdminBackupCommand::Delete(args) => {
			let prompt = format!("Delete backup '{}' ? ", args.backup);
			if !confirm(global, &prompt)? {
				return Ok(());
			}

			let response = trpc
				.call("admin.deleteBackup", json!({ "fileName": args.backup }))
				.await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
	}
}

async fn mail(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	trpc: &TrpcClient,
	command: AdminMailCommand,
) -> Result<(), CliError> {
	match command {
		AdminMailCommand::Setup(args) => {
			let mut input = serde_json::Map::new();
			input.insert("smtpHost".to_string(), Value::String(args.host));
			input.insert("smtpPort".to_string(), Value::String(args.port));
			input.insert("smtpSecure".to_string(), Value::Bool(args.secure));
			if let Some(user) = args.user {
				input.insert("smtpUsername".to_string(), Value::String(user));
			}
			if let Some(pass) = args.pass {
				input.insert("smtpPassword".to_string(), Value::String(pass));
			}
			if let Some(from) = args.from {
				input.insert("smtpEmail".to_string(), Value::String(from));
			}
			if let Some(from_name) = args.from_name {
				input.insert("smtpFromName".to_string(), Value::String(from_name));
			}

			let response = trpc.call("admin.setMail", Value::Object(input)).await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
		AdminMailCommand::Test(args) => {
			let response = trpc
				.call(
					"admin.sendTestMail",
					json!({ "type": mail_template_key_to_string(args.r#type) }),
				)
				.await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
		AdminMailCommand::Templates { command } => match command {
			AdminMailTemplatesCommand::List => {
				let keys = [
					"inviteUserTemplate",
					"inviteAdminTemplate",
					"inviteOrganizationTemplate",
					"forgotPasswordTemplate",
					"verifyEmailTemplate",
					"notificationTemplate",
					"newDeviceNotificationTemplate",
					"deviceIpChangeNotificationTemplate",
				];

				if matches!(effective.output, OutputFormat::Table) {
					for k in keys {
						println!("{k}");
					}
					return Ok(());
				}

				let value = Value::Array(keys.iter().map(|k| Value::String((*k).to_string())).collect());
				output::print_value(&value, effective.output, global.no_color)?;
				Ok(())
			}
			AdminMailTemplatesCommand::Get(args) => {
				let response = trpc
					.call("admin.getMailTemplates", json!({ "template": args.name }))
					.await?;
				print_human_or_machine(&response, effective.output, global.no_color)?;
				Ok(())
			}
			AdminMailTemplatesCommand::Set(args) => {
				let text = std::fs::read_to_string(&args.file)?;
				serde_json::from_str::<Value>(&text).map_err(|err| {
					CliError::InvalidArgument(format!("invalid template json: {err}"))
				})?;

				let response = trpc
					.call(
						"admin.setMailTemplates",
						json!({ "type": args.name, "template": text }),
					)
					.await?;
				print_human_or_machine(&response, effective.output, global.no_color)?;
				Ok(())
			}
		},
	}
}

async fn settings(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	trpc: &TrpcClient,
	command: AdminSettingsCommand,
) -> Result<(), CliError> {
	match command {
		AdminSettingsCommand::Get => {
			let response = trpc.call("settings.getAllOptions", Value::Null).await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
		AdminSettingsCommand::Update(args) => {
			if !args.enable_registration
				&& !args.disable_registration
				&& args.site_name.is_none()
				&& args.welcome_title.is_none()
				&& args.welcome_body.is_none()
			{
				return Err(CliError::InvalidArgument(
					"no update fields provided".to_string(),
				));
			}

			let mut input = serde_json::Map::new();

			if args.enable_registration {
				input.insert("enableRegistration".to_string(), Value::Bool(true));
			} else if args.disable_registration {
				input.insert("enableRegistration".to_string(), Value::Bool(false));
			}
			if let Some(site_name) = args.site_name {
				input.insert("siteName".to_string(), Value::String(site_name));
			}
			if let Some(title) = args.welcome_title {
				input.insert("welcomeMessageTitle".to_string(), Value::String(title));
			}
			if let Some(body) = args.welcome_body {
				input.insert("welcomeMessageBody".to_string(), Value::String(body));
			}

			let response = trpc
				.call("admin.updateGlobalOptions", Value::Object(input))
				.await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
	}
}

async fn invites(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	trpc: &TrpcClient,
	command: AdminInvitesCommand,
) -> Result<(), CliError> {
	match command {
		AdminInvitesCommand::List => {
			let response = trpc.call("admin.getInvitationLink", Value::Null).await?;
			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
		AdminInvitesCommand::Create(args) => {
			let secret = args.secret.unwrap_or_else(default_invite_secret);

			let mut input = serde_json::Map::new();
			input.insert("secret".to_string(), Value::String(secret));
			input.insert("expireTime".to_string(), Value::String(args.expires_min.to_string()));
			if let Some(uses) = args.uses {
				input.insert("timesCanUse".to_string(), Value::String(uses.to_string()));
			}
			if let Some(group) = args.group {
				input.insert("groupId".to_string(), Value::String(group));
			}

			let response = trpc
				.call("admin.generateInviteLink", Value::Object(input))
				.await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
		AdminInvitesCommand::Delete(args) => {
			let prompt = format!("Delete invite link '{}' ? ", args.id);
			if !confirm(global, &prompt)? {
				return Ok(());
			}
			let response = trpc
				.call("admin.deleteInvitationLink", json!({ "id": args.id }))
				.await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
	}
}

fn user_role_to_string(role: UserRole) -> &'static str {
	match role {
		UserRole::ReadOnly => "READ_ONLY",
		UserRole::User => "USER",
		UserRole::Admin => "ADMIN",
	}
}

fn mail_template_key_to_string(key: MailTemplateKeyArg) -> &'static str {
	match key {
		MailTemplateKeyArg::InviteUser => "inviteUserTemplate",
		MailTemplateKeyArg::InviteAdmin => "inviteAdminTemplate",
		MailTemplateKeyArg::InviteOrganization => "inviteOrganizationTemplate",
		MailTemplateKeyArg::ForgotPassword => "forgotPasswordTemplate",
		MailTemplateKeyArg::VerifyEmail => "verifyEmailTemplate",
		MailTemplateKeyArg::Notification => "notificationTemplate",
		MailTemplateKeyArg::NewDeviceNotification => "newDeviceNotificationTemplate",
		MailTemplateKeyArg::DeviceIpChangeNotification => "deviceIpChangeNotificationTemplate",
	}
}

fn default_invite_secret() -> String {
	let nanos = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.unwrap_or_default()
		.as_nanos();
	format!("ztnet-cli-{nanos}")
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
	)?
	.with_cookie(Some(cookie)))
}

