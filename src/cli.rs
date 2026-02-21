use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(name = "ztnet", version, about = "ZTNet CLI — manage ZeroTier networks via ZTNet")]
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

#[derive(Subcommand, Debug)]
pub enum NetworkCommand {
	List(NetworkListArgs),
	Create(NetworkCreateArgs),
	Get(NetworkGetArgs),
	Update(NetworkUpdateArgs),
	Member {
		#[command(subcommand)]
		command: NetworkMemberCommand,
	},
}

#[derive(Args, Debug)]
pub struct NetworkListArgs {
	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,

	#[arg(long)]
	pub details: bool,

	#[arg(long)]
	pub ids_only: bool,

	#[arg(long, value_name = "EXPR")]
	pub filter: Option<String>,
}

#[derive(Args, Debug)]
pub struct NetworkCreateArgs {
	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,

	#[arg(long, value_name = "NAME")]
	pub name: Option<String>,
}

#[derive(Args, Debug)]
pub struct NetworkGetArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,
}

#[derive(Args, Debug)]
pub struct NetworkUpdateArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(long, value_name = "ORG")]
	pub org: String,

	#[arg(long, value_name = "NAME")]
	pub name: Option<String>,

	#[arg(long, value_name = "TEXT")]
	pub description: Option<String>,

	#[arg(long, value_name = "MTU")]
	pub mtu: Option<String>,

	#[arg(long, conflicts_with = "public")]
	pub private: bool,

	#[arg(long, conflicts_with = "private")]
	pub public: bool,

	#[arg(long, value_name = "TEXT", conflicts_with = "flow_rule_file")]
	pub flow_rule: Option<String>,

	#[arg(long, value_name = "PATH", conflicts_with = "flow_rule")]
	pub flow_rule_file: Option<PathBuf>,

	#[arg(long, value_name = "DOMAIN")]
	pub dns_domain: Option<String>,

	#[arg(long, value_name = "IP")]
	pub dns_server: Vec<String>,

	#[arg(long, value_name = "JSON", conflicts_with = "body_file")]
	pub body: Option<String>,

	#[arg(long, value_name = "PATH", conflicts_with = "body")]
	pub body_file: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum NetworkMemberCommand {
	List(MemberListArgs),
	Get(MemberGetArgs),
	Update(MemberUpdateArgs),
	Authorize(MemberAuthorizeArgs),
	Deauthorize(MemberDeauthorizeArgs),
	#[command(alias = "stash")]
	Delete(MemberDeleteArgs),
}

#[derive(Args, Debug)]
pub struct MemberListArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,

	#[arg(long, conflicts_with = "unauthorized")]
	pub authorized: bool,

	#[arg(long, conflicts_with = "authorized")]
	pub unauthorized: bool,

	#[arg(long, value_name = "SUBSTRING")]
	pub name: Option<String>,

	#[arg(long, value_name = "NODEID")]
	pub id: Option<String>,
}

#[derive(Args, Debug)]
pub struct MemberGetArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(value_name = "MEMBER")]
	pub member: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,
}

#[derive(Args, Debug)]
pub struct MemberUpdateArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(value_name = "MEMBER")]
	pub member: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,

	#[arg(long, value_name = "NAME")]
	pub name: Option<String>,

	#[arg(long, value_name = "TEXT")]
	pub description: Option<String>,

	#[arg(long, conflicts_with = "unauthorized")]
	pub authorized: bool,

	#[arg(long, conflicts_with = "authorized")]
	pub unauthorized: bool,

	#[arg(long, value_name = "JSON", conflicts_with = "body_file")]
	pub body: Option<String>,

	#[arg(long, value_name = "PATH", conflicts_with = "body")]
	pub body_file: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct MemberAuthorizeArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(value_name = "MEMBER")]
	pub member: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,
}

#[derive(Args, Debug)]
pub struct MemberDeauthorizeArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(value_name = "MEMBER")]
	pub member: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,
}

#[derive(Args, Debug)]
pub struct MemberDeleteArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(value_name = "MEMBER")]
	pub member: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum MemberCommand {
	List(MemberListArgs),
	Get(MemberGetArgs),
	Update(MemberUpdateArgs),
	Authorize(MemberAuthorizeArgs),
	Deauthorize(MemberDeauthorizeArgs),
	#[command(alias = "stash")]
	Delete(MemberDeleteArgs),
}

#[derive(Subcommand, Debug)]
pub enum StatsCommand {
	Get,
}

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

#[derive(Args, Debug)]
pub struct CompletionArgs {
	#[arg(value_enum, value_name = "SHELL")]
	pub shell: clap_complete::Shell,
}
