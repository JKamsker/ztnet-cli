use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum StatsCommand {
	Get,
}

