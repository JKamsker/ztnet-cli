use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum PlanetCommand {
	Download(PlanetDownloadArgs),
}

#[derive(Args, Debug)]
pub struct PlanetDownloadArgs {
	#[arg(long, value_name = "PATH")]
	pub out: Option<PathBuf>,

	#[arg(long)]
	pub stdout: bool,

	#[arg(long)]
	pub force: bool,
}

