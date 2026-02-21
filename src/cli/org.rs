use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum OrgCommand {
	List(OrgListArgs),
	Get(OrgGetArgs),
	Users {
		#[command(subcommand)]
		command: OrgUsersCommand,
	},
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
}

#[derive(Args, Debug)]
pub struct OrgUsersListArgs {
	#[arg(long, value_name = "ORG")]
	pub org: String,
}

