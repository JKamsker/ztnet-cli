use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::SESSION_AUTH_LONG_ABOUT;

#[derive(Subcommand, Debug)]
pub enum NetworkCommand {
	List(NetworkListArgs),
	Create(NetworkCreateArgs),
	Get(NetworkGetArgs),
	Update(NetworkUpdateArgs),
	#[command(about = "Delete a network [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Delete(NetworkDeleteArgs),
	#[command(about = "Manage network routes [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Routes(NetworkRoutesArgs),
	#[command(about = "Manage network IP pools [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	IpPool(NetworkIpPoolArgs),
	#[command(about = "Configure network DNS [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Dns(NetworkDnsArgs),
	#[command(about = "Configure network IPv6 [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Ipv6(NetworkIpv6Args),
	#[command(about = "Configure multicast/broadcast [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Multicast(NetworkMulticastArgs),
	#[command(about = "Flow rules [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	FlowRules(NetworkFlowRulesArgs),
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

#[derive(Args, Debug)]
pub struct NetworkDeleteArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,
}

#[derive(Args, Debug)]
pub struct NetworkRoutesArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,

	#[command(subcommand)]
	pub command: NetworkRoutesCommand,
}

#[derive(Subcommand, Debug)]
pub enum NetworkRoutesCommand {
	#[command(about = "List routes [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	List,
	#[command(about = "Add a route [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Add(NetworkRoutesAddArgs),
	#[command(about = "Remove a route [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Remove(NetworkRoutesRemoveArgs),
}

#[derive(Args, Debug)]
pub struct NetworkRoutesAddArgs {
	#[arg(long, value_name = "CIDR")]
	pub destination: String,

	#[arg(long, value_name = "GATEWAY", help = "Gateway IP, or 'lan'")]
	pub via: Option<String>,
}

#[derive(Args, Debug)]
pub struct NetworkRoutesRemoveArgs {
	#[arg(long, value_name = "CIDR")]
	pub destination: String,
}

#[derive(Args, Debug)]
pub struct NetworkIpPoolArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,

	#[command(subcommand)]
	pub command: NetworkIpPoolCommand,
}

#[derive(Subcommand, Debug)]
pub enum NetworkIpPoolCommand {
	#[command(about = "List IP pools [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	List,
	#[command(about = "Add an IP pool [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Add(NetworkIpPoolChangeArgs),
	#[command(about = "Remove an IP pool [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Remove(NetworkIpPoolChangeArgs),
}

#[derive(Args, Debug)]
pub struct NetworkIpPoolChangeArgs {
	#[arg(long, value_name = "IP", required_unless_present = "cidr")]
	pub start: Option<String>,

	#[arg(long, value_name = "IP", required_unless_present = "cidr")]
	pub end: Option<String>,

	#[arg(long, value_name = "CIDR", conflicts_with_all = ["start", "end"])]
	pub cidr: Option<String>,
}

#[derive(Args, Debug)]
pub struct NetworkDnsArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,

	#[arg(long, value_name = "DOMAIN", conflicts_with = "clear")]
	pub domain: Option<String>,

	#[arg(long, value_delimiter = ',', value_name = "IP", conflicts_with = "clear")]
	pub servers: Vec<String>,

	#[arg(long, help = "Clear DNS settings", conflicts_with_all = ["domain", "servers"])]
	pub clear: bool,
}

#[derive(Args, Debug)]
pub struct NetworkIpv6Args {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,

	#[arg(long = "6plane", conflicts_with = "no_6plane")]
	pub sixplane: bool,

	#[arg(long = "no-6plane", conflicts_with = "sixplane")]
	pub no_6plane: bool,

	#[arg(long, conflicts_with = "no_rfc4193")]
	pub rfc4193: bool,

	#[arg(long = "no-rfc4193", conflicts_with = "rfc4193")]
	pub no_rfc4193: bool,

	#[arg(long, conflicts_with = "no_zt")]
	pub zt: bool,

	#[arg(long = "no-zt", conflicts_with = "zt")]
	pub no_zt: bool,
}

#[derive(Args, Debug)]
pub struct NetworkMulticastArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,

	#[arg(long, value_name = "N")]
	pub limit: Option<u32>,

	#[arg(long = "enable", alias = "enable-broadcast", conflicts_with = "disable")]
	pub enable: bool,

	#[arg(long = "disable", alias = "disable-broadcast", conflicts_with = "enable")]
	pub disable: bool,
}

#[derive(Args, Debug)]
pub struct NetworkFlowRulesArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[command(subcommand)]
	pub command: NetworkFlowRulesCommand,
}

#[derive(Subcommand, Debug)]
pub enum NetworkFlowRulesCommand {
	#[command(about = "Get flow rules [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Get(NetworkFlowRulesGetArgs),
}

#[derive(Args, Debug)]
pub struct NetworkFlowRulesGetArgs {
	#[arg(long)]
	pub reset: bool,
}

#[derive(Subcommand, Debug)]
pub enum NetworkMemberCommand {
	List(MemberListArgs),
	Get(MemberGetArgs),
	Update(MemberUpdateArgs),
	Authorize(MemberAuthorizeArgs),
	Deauthorize(MemberDeauthorizeArgs),
	#[command(about = "Add a member by node id [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Add(MemberAddArgs),
	#[command(about = "Manage member tags [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Tags(MemberTagsArgs),
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

#[derive(Args, Debug)]
pub struct MemberAddArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(value_name = "NODE_ID")]
	pub node_id: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,
}

#[derive(Args, Debug)]
pub struct MemberTagsArgs {
	#[arg(value_name = "NETWORK")]
	pub network: String,

	#[arg(value_name = "MEMBER")]
	pub member: String,

	#[arg(long, value_name = "ORG")]
	pub org: Option<String>,

	#[command(subcommand)]
	pub command: MemberTagsCommand,
}

#[derive(Subcommand, Debug)]
pub enum MemberTagsCommand {
	#[command(about = "List tags [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	List,
	#[command(about = "Set tags [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Set(MemberTagsSetArgs),
}

#[derive(Args, Debug)]
pub struct MemberTagsSetArgs {
	#[arg(long, value_name = "JSON")]
	pub tags: String,
}

#[derive(Subcommand, Debug)]
pub enum MemberCommand {
	List(MemberListArgs),
	Get(MemberGetArgs),
	Update(MemberUpdateArgs),
	Authorize(MemberAuthorizeArgs),
	Deauthorize(MemberDeauthorizeArgs),
	#[command(about = "Add a member by node id [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Add(MemberAddArgs),
	#[command(about = "Manage member tags [session auth]", long_about = SESSION_AUTH_LONG_ABOUT)]
	Tags(MemberTagsArgs),
	#[command(alias = "stash")]
	Delete(MemberDeleteArgs),
}
