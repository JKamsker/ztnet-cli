use reqwest::Method;
use serde_json::{json, Value};

use crate::cli::{GlobalOpts, TrpcCommand};
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::http::HttpClient;
use crate::output;

use super::common::{load_config_store, print_human_or_machine};
use super::trpc_client::cookie_from_effective;

pub(super) async fn run(global: &GlobalOpts, command: TrpcCommand) -> Result<(), CliError> {
	let (_config_path, cfg) = load_config_store()?;
	let effective = resolve_effective_config(global, &cfg)?;

	let client = HttpClient::new(
		&effective.host,
		None,
		effective.timeout,
		effective.retries,
		global.dry_run,
	)?;

	match command {
		TrpcCommand::List => {
			let value = json!({
				"routers": {
					"network": ["getUserNetworks", "getNetworkById", "deleteNetwork", "ipv6", "enableIpv4AutoAssign", "managedRoutes", "easyIpAssignment"],
					"networkMember": ["getAll", "getMemberById", "create", "Update", "Tags", "UpdateDatabaseOnly", "stash", "delete", "getMemberAnotations", "removeMemberAnotations", "bulkDeleteStashed"],
					"auth": ["register", "me", "update", "validateResetPasswordToken", "passwordResetLink", "changePasswordFromJwt", "sendVerificationEmail", "validateEmailVerificationToken", "updateUserOptions", "setZtApi", "setLocalZt", "getApiToken", "addApiToken", "deleteApiToken", "deleteUserDevice"],
					"mfaAuth": ["mfaValidateToken", "mfaResetLink", "mfaResetValidation", "validateRecoveryToken"],
					"admin": ["updateUser", "deleteUser", "createUser", "getUser", "getUsers", "generateInviteLink", "getInvitationLink", "deleteInvitationLink", "getControllerStats", "getAllOptions", "changeRole", "updateGlobalOptions", "getMailTemplates", "setMail", "setMailTemplates", "getDefaultMailTemplate", "sendTestMail", "unlinkedNetwork", "assignNetworkToUser", "addUserGroup", "getUserGroups", "deleteUserGroup", "assignUserGroup", "getIdentity", "getPlanet", "makeWorld", "resetWorld", "createBackup", "downloadBackup", "listBackups", "deleteBackup", "restoreBackup", "uploadBackup"],
					"settings": ["getAllOptions", "getPublicOptions", "getAdminOptions"],
					"org": ["createOrg", "deleteOrg", "updateMeta", "getOrgIdbyUserid", "getAllOrg", "getOrgUserRoleById", "getPlatformUsers", "getOrgUsers", "getOrgById", "createOrgNetwork", "changeUserRole", "sendMessage", "getMessages", "markMessagesAsRead", "getOrgNotifications", "addUser", "leave", "getLogs", "preValidateUserInvite", "generateInviteLink", "resendInvite", "inviteUserByMail", "deleteInvite", "getInvites", "transferNetworkOwnership", "deleteOrgWebhooks", "addOrgWebhooks", "getOrgWebhooks", "updateOrganizationSettings", "getOrganizationSettings", "updateOrganizationNotificationSettings", "getOrganizationNotificationTemplate", "getDefaultOrganizationNotificationTemplate", "updateOrganizationNotificationTemplate", "sendTestOrganizationNotification"],
					"public": ["registrationAllowed", "getWelcomeMessage"]
				}
			});

			print_human_or_machine(&value, effective.output, global.no_color)?;
			Ok(())
		}
		TrpcCommand::Call(args) => {
			let input = if let Some(input) = args.input {
				serde_json::from_str::<Value>(&input).map_err(|err| {
					CliError::InvalidArgument(format!("invalid --input json: {err}"))
				})?
			} else if let Some(path) = args.input_file {
				let text = std::fs::read_to_string(&path)?;
				serde_json::from_str::<Value>(&text).map_err(|err| {
					CliError::InvalidArgument(format!("invalid --input-file json: {err}"))
				})?
			} else {
				Value::Null
			};

			let cookie = if let Some(cookie) = args.cookie {
				Some(cookie)
			} else if let Some(path) = args.cookie_file {
				Some(std::fs::read_to_string(&path)?.trim().to_string())
			} else {
				cookie_from_effective(&effective)
			};

			let mut headers = reqwest::header::HeaderMap::new();
			if let Some(cookie) = cookie {
				headers.insert(
					reqwest::header::COOKIE,
					reqwest::header::HeaderValue::from_str(cookie.trim()).map_err(|_| {
						CliError::InvalidArgument("invalid cookie header value".to_string())
					})?,
				);
			}

			let body = json!({ "0": { "json": input } });
			let path = format!("/api/trpc/{}?batch=1", args.procedure);

			let response = client
				.request_json(Method::POST, &path, Some(body), headers, false)
				.await?;

			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
	}
}
