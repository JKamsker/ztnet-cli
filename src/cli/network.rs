use std::path::PathBuf;

use clap::{Args, Subcommand};

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

