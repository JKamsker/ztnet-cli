use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

#[derive(Subcommand, Debug)]
pub enum ExportCommand {
	Hosts(ExportHostsArgs),
}

#[derive(ValueEnum, Debug, Clone, Copy, Default)]
pub enum ExportHostsFormat {
	#[default]
	Hosts,
	Csv,
	Json,
}

impl std::fmt::Display for ExportHostsFormat {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let value = match self {
			ExportHostsFormat::Hosts => "hosts",
			ExportHostsFormat::Csv => "csv",
			ExportHostsFormat::Json => "json",
		};
		write!(f, "{value}")
	}
}

#[derive(Args, Debug)]
pub struct ExportHostsArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,

	#[arg(long, value_name = "DOMAIN")]
	pub zone: String,

	#[arg(long, value_name = "PATH")]
	pub out: Option<PathBuf>,

	#[arg(long)]
	pub authorized_only: bool,

	#[arg(long)]
	pub include_unauthorized: bool,

	#[arg(long, value_enum, default_value_t = ExportHostsFormat::Hosts)]
	pub format: ExportHostsFormat,
}

