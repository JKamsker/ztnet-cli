use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
	Path,
	Get(ConfigGetArgs),
	Set(ConfigSetArgs),
	Unset(ConfigUnsetArgs),
	List,
	Context {
		#[command(subcommand)]
		command: ConfigContextCommand,
	},
}

#[derive(Args, Debug)]
pub struct ConfigGetArgs {
	#[arg(value_name = "KEY")]
	pub key: String,
}

#[derive(Args, Debug)]
pub struct ConfigSetArgs {
	#[arg(value_name = "KEY")]
	pub key: String,

	#[arg(value_name = "VALUE")]
	pub value: String,
}

#[derive(Args, Debug)]
pub struct ConfigUnsetArgs {
	#[arg(value_name = "KEY")]
	pub key: String,
}

#[derive(Subcommand, Debug)]
pub enum ConfigContextCommand {
	Show,
	Set(ConfigContextSetArgs),
	Clear,
}

#[derive(Args, Debug)]
pub struct ConfigContextSetArgs {
	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,

	#[arg(long, value_name = "NETWORK")]
	pub network: Option<String>,
}

