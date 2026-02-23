use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum AuthCommand {
	SetToken(AuthSetTokenArgs),
	UnsetToken(AuthUnsetTokenArgs),
	Login(AuthLoginArgs),
	Logout(AuthLogoutArgs),
	Show,
	Test(AuthTestArgs),
	Profiles {
		#[command(subcommand)]
		command: AuthProfilesCommand,
	},
	Hosts {
		#[command(subcommand)]
		command: AuthHostsCommand,
	},
}

#[derive(Args, Debug)]
pub struct AuthSetTokenArgs {
	#[arg(long, value_name = "NAME")]
	pub profile: Option<String>,

	#[arg(long, help = "Read token from STDIN (avoids shell history)")]
	pub stdin: bool,

	#[arg(long, help = "Skip token validation against the server")]
	pub no_validate: bool,

	#[arg(value_name = "TOKEN")]
	pub token: Option<String>,
}

#[derive(Args, Debug)]
pub struct AuthUnsetTokenArgs {
	#[arg(long, value_name = "NAME")]
	pub profile: Option<String>,
}

#[derive(Args, Debug)]
pub struct AuthLoginArgs {
	#[arg(long, value_name = "NAME")]
	pub profile: Option<String>,

	#[arg(long, value_name = "EMAIL", env = "ZTNET_EMAIL")]
	pub email: Option<String>,

	#[arg(long, value_name = "PASSWORD", env = "ZTNET_PASSWORD", conflicts_with = "password_stdin")]
	pub password: Option<String>,

	#[arg(long, help = "Read password from STDIN (avoids shell history)", conflicts_with = "password")]
	pub password_stdin: bool,

	#[arg(long, value_name = "CODE")]
	pub totp: Option<String>,
}

#[derive(Args, Debug)]
pub struct AuthLogoutArgs {
	#[arg(long, value_name = "NAME")]
	pub profile: Option<String>,
}

#[derive(Args, Debug)]
pub struct AuthTestArgs {
	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum AuthProfilesCommand {
	List,
	Use(AuthProfilesUseArgs),
}

#[derive(Args, Debug)]
pub struct AuthProfilesUseArgs {
	#[arg(value_name = "NAME")]
	pub name: String,
}

#[derive(Subcommand, Debug)]
pub enum AuthHostsCommand {
	List,
	SetDefault(AuthHostsSetDefaultArgs),
	UnsetDefault(AuthHostsUnsetDefaultArgs),
}

#[derive(Args, Debug)]
pub struct AuthHostsSetDefaultArgs {
	#[arg(value_name = "HOST")]
	pub host: String,

	#[arg(value_name = "PROFILE")]
	pub profile: Option<String>,
}

#[derive(Args, Debug)]
pub struct AuthHostsUnsetDefaultArgs {
	#[arg(value_name = "HOST")]
	pub host: String,
}

