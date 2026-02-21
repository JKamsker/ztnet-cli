use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum ApiCommand {
	Request(ApiRequestArgs),
	Get(ApiGetArgs),
	Post(ApiPostArgs),
	Delete(ApiDeleteArgs),
}

#[derive(Args, Debug)]
pub struct ApiRequestArgs {
	#[arg(value_name = "METHOD")]
	pub method: String,

	#[arg(value_name = "PATH")]
	pub path: String,

	#[arg(long, value_name = "JSON", conflicts_with = "body_file")]
	pub body: Option<String>,

	#[arg(long, value_name = "PATH", conflicts_with = "body")]
	pub body_file: Option<PathBuf>,

	#[arg(long, value_name = "K:V")]
	pub header: Vec<String>,

	#[arg(long)]
	pub no_auth: bool,

	#[arg(long)]
	pub raw: bool,
}

#[derive(Args, Debug)]
pub struct ApiGetArgs {
	#[arg(value_name = "PATH")]
	pub path: String,
}

#[derive(Args, Debug)]
pub struct ApiPostArgs {
	#[arg(value_name = "PATH")]
	pub path: String,

	#[arg(long, value_name = "JSON", conflicts_with = "body_file")]
	pub body: Option<String>,

	#[arg(long, value_name = "PATH", conflicts_with = "body")]
	pub body_file: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ApiDeleteArgs {
	#[arg(value_name = "PATH")]
	pub path: String,
}

