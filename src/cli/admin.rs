use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

use super::SESSION_AUTH_LONG_ABOUT;

#[derive(Subcommand, Debug)]
pub enum AdminCommand {
	#[command(about = "Manage users [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Users {
		#[command(subcommand)]
		command: AdminUsersCommand,
	},
	#[command(about = "Manage backups [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Backup {
		#[command(subcommand)]
		command: AdminBackupCommand,
	},
	#[command(about = "Configure mail [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Mail {
		#[command(subcommand)]
		command: AdminMailCommand,
	},
	#[command(about = "Global settings [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Settings {
		#[command(subcommand)]
		command: AdminSettingsCommand,
	},
	#[command(about = "Invitation links [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Invites {
		#[command(subcommand)]
		command: AdminInvitesCommand,
	},
}

#[derive(Subcommand, Debug)]
pub enum AdminUsersCommand {
	#[command(about = "List users [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	List(AdminUsersListArgs),
	#[command(about = "Get user [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Get(AdminUsersGetArgs),
	#[command(about = "Delete user [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Delete(AdminUsersDeleteArgs),
	#[command(about = "Update user [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Update(AdminUsersUpdateArgs),
}

#[derive(Args, Debug)]
pub struct AdminUsersListArgs {
	#[arg(long, help = "List only admins")]
	pub admins: bool,
}

#[derive(Args, Debug)]
pub struct AdminUsersGetArgs {
	#[arg(value_name = "USER")]
	pub user: String,
}

#[derive(Args, Debug)]
pub struct AdminUsersDeleteArgs {
	#[arg(value_name = "USER")]
	pub user: String,
}

#[derive(Args, Debug)]
pub struct AdminUsersUpdateArgs {
	#[arg(value_name = "USER")]
	pub user: String,

	#[arg(long, value_name = "ROLE")]
	pub role: Option<UserRole>,

	#[arg(long, conflicts_with = "inactive")]
	pub active: bool,

	#[arg(long, conflicts_with = "active")]
	pub inactive: bool,
}

#[derive(Subcommand, Debug)]
pub enum AdminBackupCommand {
	#[command(about = "List backups [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	List,
	#[command(about = "Create backup [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Create(AdminBackupCreateArgs),
	#[command(about = "Download backup [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Download(AdminBackupDownloadArgs),
	#[command(about = "Restore backup [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Restore(AdminBackupRestoreArgs),
	#[command(about = "Delete backup [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Delete(AdminBackupDeleteArgs),
}

#[derive(Args, Debug)]
pub struct AdminBackupCreateArgs {
	#[arg(long, help = "Do not include database")]
	pub no_database: bool,

	#[arg(long, help = "Do not include ZeroTier folder")]
	pub no_zerotier: bool,

	#[arg(long, value_name = "NAME")]
	pub name: Option<String>,
}

#[derive(Args, Debug)]
pub struct AdminBackupDownloadArgs {
	#[arg(value_name = "BACKUP")]
	pub backup: String,

	#[arg(long, value_name = "PATH")]
	pub out: PathBuf,
}

#[derive(Args, Debug)]
pub struct AdminBackupRestoreArgs {
	#[arg(value_name = "BACKUP")]
	pub backup: String,

	#[arg(long, help = "Do not restore database")]
	pub no_database: bool,

	#[arg(long, help = "Do not restore ZeroTier folder")]
	pub no_zerotier: bool,
}

#[derive(Args, Debug)]
pub struct AdminBackupDeleteArgs {
	#[arg(value_name = "BACKUP")]
	pub backup: String,
}

#[derive(Subcommand, Debug)]
pub enum AdminMailCommand {
	#[command(about = "Set SMTP settings [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Setup(AdminMailSetupArgs),
	#[command(about = "Send a test mail [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Test(AdminMailTestArgs),
	#[command(about = "Manage templates [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Templates {
		#[command(subcommand)]
		command: AdminMailTemplatesCommand,
	},
}

#[derive(Args, Debug)]
pub struct AdminMailSetupArgs {
	#[arg(long, value_name = "HOST")]
	pub host: String,

	#[arg(long, value_name = "PORT")]
	pub port: String,

	#[arg(long, value_name = "USER")]
	pub user: Option<String>,

	#[arg(long, value_name = "PASS")]
	pub pass: Option<String>,

	#[arg(long, value_name = "EMAIL", help = "From address (smtpEmail)")]
	pub from: Option<String>,

	#[arg(long, value_name = "NAME", help = "From name (smtpFromName)")]
	pub from_name: Option<String>,

	#[arg(long, help = "Use implicit TLS (secure)")]
	pub secure: bool,
}

#[derive(Args, Debug)]
pub struct AdminMailTestArgs {
	#[arg(long, value_name = "TYPE")]
	pub r#type: MailTemplateKeyArg,
}

#[derive(Subcommand, Debug)]
pub enum AdminMailTemplatesCommand {
	#[command(about = "List template keys [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	List,
	#[command(about = "Get template [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Get(AdminMailTemplatesGetArgs),
	#[command(about = "Set template [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Set(AdminMailTemplatesSetArgs),
}

#[derive(Args, Debug)]
pub struct AdminMailTemplatesGetArgs {
	#[arg(value_name = "NAME")]
	pub name: String,
}

#[derive(Args, Debug)]
pub struct AdminMailTemplatesSetArgs {
	#[arg(value_name = "NAME")]
	pub name: String,

	#[arg(long, value_name = "PATH")]
	pub file: PathBuf,
}

#[derive(Subcommand, Debug)]
pub enum AdminSettingsCommand {
	#[command(about = "Get settings [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Get,
	#[command(about = "Update settings [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Update(AdminSettingsUpdateArgs),
}

#[derive(Args, Debug)]
pub struct AdminSettingsUpdateArgs {
	#[arg(long, conflicts_with = "disable_registration")]
	pub enable_registration: bool,

	#[arg(long, conflicts_with = "enable_registration")]
	pub disable_registration: bool,

	#[arg(long, value_name = "NAME")]
	pub site_name: Option<String>,

	#[arg(long, value_name = "TEXT")]
	pub welcome_title: Option<String>,

	#[arg(long, value_name = "TEXT")]
	pub welcome_body: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum AdminInvitesCommand {
	#[command(about = "List invitation links [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	List,
	#[command(about = "Create invitation link [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Create(AdminInvitesCreateArgs),
	#[command(about = "Delete invitation link [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Delete(AdminInvitesDeleteArgs),
}

#[derive(Args, Debug)]
pub struct AdminInvitesCreateArgs {
	#[arg(long, value_name = "TEXT")]
	pub secret: Option<String>,

	#[arg(long, value_name = "MINUTES", default_value = "60")]
	pub expires_min: u32,

	#[arg(long, value_name = "N")]
	pub uses: Option<u32>,

	#[arg(long, value_name = "GROUP")]
	pub group: Option<String>,
}

#[derive(Args, Debug)]
pub struct AdminInvitesDeleteArgs {
	#[arg(value_name = "ID")]
	pub id: u64,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum UserRole {
	#[value(name = "read-only")]
	ReadOnly,
	User,
	Admin,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum MailTemplateKeyArg {
	InviteUser,
	InviteAdmin,
	InviteOrganization,
	ForgotPassword,
	VerifyEmail,
	Notification,
	NewDeviceNotification,
	DeviceIpChangeNotification,
}

