mod app;
mod cli;
mod config;
mod context;
mod error;
mod host;
mod http;
mod multi_base;
mod output;

use clap::Parser;

#[tokio::main]
async fn main() {
	let cli = cli::Cli::parse();

	if let Err(err) = app::run(cli).await {
		let code = err.exit_code();
		if code != 0 {
			eprintln!("{err}");
		}
		std::process::exit(code);
	}
}
