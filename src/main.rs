mod cli;
mod config;
mod context;
mod error;
mod http;

use clap::{CommandFactory, Parser};

fn main() {
	let cli = cli::Cli::parse();

	match cli.command {
		cli::Command::Completion(args) => {
			let mut cmd = cli::Cli::command();
			clap_complete::generate(args.shell, &mut cmd, "ztnet", &mut std::io::stdout());
		}
		_ => {
			let config_path = match config::default_config_path() {
				Ok(path) => path,
				Err(err) => {
					eprintln!("{err}");
					std::process::exit(1);
				}
			};

			let config = match config::load_config(&config_path) {
				Ok(cfg) => cfg,
				Err(err) => {
					eprintln!("{err}");
					std::process::exit(1);
				}
			};

			if let Err(err) = context::resolve_effective_config(&cli.global, &config) {
				eprintln!("{err}");
				std::process::exit(1);
			}

			eprintln!("Not implemented yet.");
			std::process::exit(1);
		}
	}
}
