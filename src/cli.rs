mod api;
mod admin;
mod auth;
mod completion;
mod config_cmd;
mod export;
mod network;
mod org;
mod planet;
mod stats;
mod trpc;
mod user;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

pub use api::*;
pub use admin::*;
pub use auth::*;
pub use completion::*;
pub use config_cmd::*;
pub use export::*;
pub use network::*;
pub use org::*;
pub use planet::*;
pub use stats::*;
pub use trpc::*;
pub use user::*;

pub(crate) const SESSION_AUTH_LONG_ABOUT: &str = "This command requires session authentication (email/password).\nRun `ztnet auth login` first.\n\nAPI tokens are not supported for this operation.";

#[derive(Parser, Debug)]
#[command(
	name = "ztnet",
	version,
	about = "ZTNet CLI — manage ZeroTier networks via ZTNet"
)]
pub struct Cli {
	#[command(flatten)]
	pub global: GlobalOpts,

	#[command(subcommand)]
	pub command: Command,
}

#[derive(Args, Debug, Clone)]
pub struct GlobalOpts {
	#[arg(
		short = 'H',
		long,
		value_name = "URL",
		help = "ZTNet base URL (e.g. http://localhost:3000)"
	)]
	pub host: Option<String>,

	#[arg(short = 't', long, value_name = "TOKEN", help = "API token (x-ztnet-auth)")]
	pub token: Option<String>,

	#[arg(long, value_name = "NAME")]
	pub profile: Option<String>,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,

	#[arg(long, value_name = "NETWORK")]
	pub network: Option<String>,

	#[arg(long, help = "Output JSON (shortcut for --output json)")]
	pub json: bool,

	#[arg(short = 'o', long, value_name = "FORMAT")]
	pub output: Option<OutputFormat>,

	#[arg(long, help = "Disable ANSI colors")]
	pub no_color: bool,

	#[arg(long, help = "Only print machine output (no prompts)")]
	pub quiet: bool,

	#[arg(short = 'v', long, action = clap::ArgAction::Count)]
	pub verbose: u8,

	#[arg(long, value_name = "DURATION")]
	pub timeout: Option<String>,

	#[arg(long, value_name = "N")]
	pub retries: Option<u32>,

	#[arg(long, help = "Print the HTTP request and exit (no network calls)")]
	pub dry_run: bool,

	#[arg(short = 'y', long, help = "Skip confirmation prompts")]
	pub yes: bool,
}

#[derive(ValueEnum, Serialize, Deserialize, Debug, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
	#[default]
	Table,
	Json,
	Yaml,
	Raw,
}

impl std::fmt::Display for OutputFormat {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let value = match self {
			OutputFormat::Table => "table",
			OutputFormat::Json => "json",
			OutputFormat::Yaml => "yaml",
			OutputFormat::Raw => "raw",
		};
		write!(f, "{value}")
	}
}

#[derive(Subcommand, Debug)]
pub enum Command {
	Auth {
		#[command(subcommand)]
		command: AuthCommand,
	},
	#[command(about = "Admin commands [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Admin {
		#[command(subcommand)]
		command: AdminCommand,
	},
	Config {
		#[command(subcommand)]
		command: ConfigCommand,
	},
	User {
		#[command(subcommand)]
		command: UserCommand,
	},
	Org {
		#[command(subcommand)]
		command: OrgCommand,
	},
	Network {
		#[command(subcommand)]
		command: NetworkCommand,
	},
	Member {
		#[command(subcommand)]
		command: MemberCommand,
	},
	Stats {
		#[command(subcommand)]
		command: StatsCommand,
	},
	Planet {
		#[command(subcommand)]
		command: PlanetCommand,
	},
	Export {
		#[command(subcommand)]
		command: ExportCommand,
	},
	Api {
		#[command(subcommand)]
		command: ApiCommand,
	},
	Trpc {
		#[command(subcommand)]
		command: TrpcCommand,
	},
	Completion(CompletionArgs),
}
