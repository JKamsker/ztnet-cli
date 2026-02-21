use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum AuthCommand {
	SetToken(AuthSetTokenArgs),
	UnsetToken(AuthUnsetTokenArgs),
	Show,
	Test(AuthTestArgs),
	Profiles {
		#[command(subcommand)]
		command: AuthProfilesCommand,
	},
}

#[derive(Args, Debug)]
pub struct AuthSetTokenArgs {
	#[arg(long, value_name = "NAME")]
	pub profile: Option<String>,

	#[arg(long, help = "Read token from STDIN (avoids shell history)")]
	pub stdin: bool,

	#[arg(value_name = "TOKEN")]
	pub token: Option<String>,
}

#[derive(Args, Debug)]
pub struct AuthUnsetTokenArgs {
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

