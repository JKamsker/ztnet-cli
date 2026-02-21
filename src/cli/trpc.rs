use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum TrpcCommand {
	List,
	Call(TrpcCallArgs),
}

#[derive(Args, Debug)]
pub struct TrpcCallArgs {
	#[arg(value_name = "ROUTER.PROCEDURE")]
	pub procedure: String,

	#[arg(long, value_name = "JSON", conflicts_with = "input_file")]
	pub input: Option<String>,

	#[arg(long, value_name = "PATH", conflicts_with = "input")]
	pub input_file: Option<PathBuf>,

	#[arg(long, value_name = "COOKIE", conflicts_with = "cookie_file")]
	pub cookie: Option<String>,

	#[arg(long, value_name = "PATH", conflicts_with = "cookie")]
	pub cookie_file: Option<PathBuf>,
}

