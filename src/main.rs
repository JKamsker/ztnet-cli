mod cli;

use clap::{CommandFactory, Parser};

fn main() {
	let cli = cli::Cli::parse();

	match cli.command {
		cli::Command::Completion(args) => {
			let mut cmd = cli::Cli::command();
			clap_complete::generate(args.shell, &mut cmd, "ztnet", &mut std::io::stdout());
		}
		_ => {
			eprintln!("Not implemented yet.");
			std::process::exit(1);
		}
	}
}
