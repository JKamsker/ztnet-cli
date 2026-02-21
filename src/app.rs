mod api;
mod admin;
mod auth;
mod common;
mod config_cmd;
mod export;
mod member;
mod network;
mod network_trpc;
mod org;
mod planet;
mod resolve;
mod stats;
mod trpc;
mod trpc_client;
mod trpc_resolve;
mod user;

use clap::CommandFactory;

use crate::cli::{Cli, Command};
use crate::error::CliError;

pub async fn run(cli: Cli) -> Result<(), CliError> {
	let Cli { global, command } = cli;

	match command {
		Command::Completion(args) => {
			let mut cmd = Cli::command();
			clap_complete::generate(args.shell, &mut cmd, "ztnet", &mut std::io::stdout());
			Ok(())
		}
		Command::Auth { command } => auth::run(&global, command).await,
		Command::Admin { command } => admin::run(&global, command).await,
		Command::Config { command } => config_cmd::run(&global, command).await,
		Command::User { command } => user::run(&global, command).await,
		Command::Org { command } => org::run(&global, command).await,
		Command::Network { command } => network::run(&global, command).await,
		Command::Member { command } => member::run_alias(&global, command).await,
		Command::Stats { command } => stats::run(&global, command).await,
		Command::Planet { command } => planet::run(&global, command).await,
		Command::Export { command } => export::run(&global, command).await,
		Command::Api { command } => api::run(&global, command).await,
		Command::Trpc { command } => trpc::run(&global, command).await,
	}
}
