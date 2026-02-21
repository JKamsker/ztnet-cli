use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum UserCommand {
	Create(UserCreateArgs),
}

#[derive(Args, Debug)]
pub struct UserCreateArgs {
	#[arg(long, value_name = "EMAIL")]
	pub email: String,

	#[arg(long, value_name = "PASSWORD")]
	pub password: String,

	#[arg(long, value_name = "NAME")]
	pub name: String,

	#[arg(long, value_name = "ISO8601")]
	pub expires_at: Option<String>,

	#[arg(long)]
	pub generate_api_token: bool,

	#[arg(long)]
	pub store_token: bool,

	#[arg(long)]
	pub print_token: bool,

	#[arg(long, help = "Force no x-ztnet-auth header (bootstrap attempt)")]
	pub no_auth: bool,
}

