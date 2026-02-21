mod app;
mod cli;
mod config;
mod context;
mod error;
mod http;
mod output;

use clap::Parser;

#[tokio::main]
async fn main() {
	let cli = cli::Cli::parse();

	if let Err(err) = app::run(cli).await {
		eprintln!("{err}");
		std::process::exit(err.exit_code());
	}
}
