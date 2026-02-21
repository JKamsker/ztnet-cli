use clap::{Args, Subcommand, ValueEnum};

use super::SESSION_AUTH_LONG_ABOUT;

#[derive(Subcommand, Debug)]
pub enum OrgCommand {
	List(OrgListArgs),
	Get(OrgGetArgs),
	Users {
		#[command(subcommand)]
		command: OrgUsersCommand,
	},
	#[command(about = "Manage org invites [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Invite {
		#[command(subcommand)]
		command: OrgInviteCommand,
	},
	#[command(about = "Org settings [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Settings {
		#[command(subcommand)]
		command: OrgSettingsCommand,
	},
	#[command(about = "Org webhooks [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Webhooks {
		#[command(subcommand)]
		command: OrgWebhooksCommand,
	},
	#[command(about = "Org logs [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Logs(OrgLogsArgs),
}

#[derive(Args, Debug)]
pub struct OrgListArgs {
	#[arg(long)]
	pub details: bool,

	#[arg(long)]
	pub ids_only: bool,
}

#[derive(Args, Debug)]
pub struct OrgGetArgs {
	#[arg(value_name = "ORG")]
	pub org: String,
}

#[derive(Subcommand, Debug)]
pub enum OrgUsersCommand {
	List(OrgUsersListArgs),
	#[command(about = "Add user to org [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Add(OrgUsersAddArgs),
	#[command(about = "Change user role [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Role(OrgUsersRoleArgs),
}

#[derive(Args, Debug)]
pub struct OrgUsersListArgs {
	#[arg(long, value_name = "ORG")]
	pub org: String,
}

#[derive(Args, Debug)]
pub struct OrgUsersAddArgs {
	#[arg(value_name = "ORG")]
	pub org: String,

	#[arg(long, value_name = "EMAIL")]
	pub email: String,

	#[arg(long, value_name = "ROLE", default_value = "user")]
	pub role: OrgRole,
}

#[derive(Args, Debug)]
pub struct OrgUsersRoleArgs {
	#[arg(value_name = "ORG")]
	pub org: String,

	#[arg(value_name = "USER")]
	pub user: String,

	#[arg(long, value_name = "ROLE")]
	pub role: OrgRole,
}

#[derive(Subcommand, Debug)]
pub enum OrgInviteCommand {
	#[command(about = "Create invite link [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Create(OrgInviteCreateArgs),
	#[command(about = "List invites [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	List(OrgInviteListArgs),
	#[command(about = "Delete invite [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Delete(OrgInviteDeleteArgs),
	#[command(about = "Send invite email [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Send(OrgInviteSendArgs),
}

#[derive(Args, Debug)]
pub struct OrgInviteCreateArgs {
	#[arg(value_name = "ORG")]
	pub org: String,

	#[arg(long, value_name = "EMAIL")]
	pub email: String,

	#[arg(long, value_name = "ROLE", default_value = "user")]
	pub role: OrgRole,
}

#[derive(Args, Debug)]
pub struct OrgInviteSendArgs {
	#[arg(value_name = "ORG")]
	pub org: String,

	#[arg(long, value_name = "EMAIL")]
	pub email: String,

	#[arg(long, value_name = "ROLE", default_value = "user")]
	pub role: OrgRole,
}

#[derive(Args, Debug)]
pub struct OrgInviteListArgs {
	#[arg(value_name = "ORG")]
	pub org: String,
}

#[derive(Args, Debug)]
pub struct OrgInviteDeleteArgs {
	#[arg(value_name = "ORG")]
	pub org: String,

	#[arg(value_name = "INVITE")]
	pub invite: String,
}

#[derive(Subcommand, Debug)]
pub enum OrgSettingsCommand {
	#[command(about = "Get org settings [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Get(OrgSettingsGetArgs),
	#[command(about = "Update org settings [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Update(OrgSettingsUpdateArgs),
}

#[derive(Args, Debug)]
pub struct OrgSettingsGetArgs {
	#[arg(value_name = "ORG")]
	pub org: String,
}

#[derive(Args, Debug)]
pub struct OrgSettingsUpdateArgs {
	#[arg(value_name = "ORG")]
	pub org: String,

	#[arg(long, conflicts_with = "no_rename_node_globally")]
	pub rename_node_globally: bool,

	#[arg(long = "no-rename-node-globally", conflicts_with = "rename_node_globally")]
	pub no_rename_node_globally: bool,
}

#[derive(Subcommand, Debug)]
pub enum OrgWebhooksCommand {
	#[command(about = "List webhooks [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	List(OrgWebhooksListArgs),
	#[command(about = "Add webhook [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Add(OrgWebhooksAddArgs),
	#[command(about = "Delete webhook [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Delete(OrgWebhooksDeleteArgs),
}

#[derive(Args, Debug)]
pub struct OrgWebhooksListArgs {
	#[arg(value_name = "ORG")]
	pub org: String,
}

#[derive(Args, Debug)]
pub struct OrgWebhooksAddArgs {
	#[arg(value_name = "ORG")]
	pub org: String,

	#[arg(long, value_name = "URL")]
	pub url: String,

	#[arg(long, value_name = "NAME")]
	pub name: String,

	#[arg(long, value_name = "EVENT")]
	pub event: Vec<String>,
}

#[derive(Args, Debug)]
pub struct OrgWebhooksDeleteArgs {
	#[arg(value_name = "ORG")]
	pub org: String,

	#[arg(value_name = "WEBHOOK")]
	pub webhook: String,
}

#[derive(Args, Debug)]
pub struct OrgLogsArgs {
	#[arg(value_name = "ORG")]
	pub org: String,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum OrgRole {
	#[value(name = "read-only")]
	ReadOnly,
	User,
	Admin,
}
