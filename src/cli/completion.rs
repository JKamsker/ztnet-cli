use clap::Args;

#[derive(Args, Debug)]
pub struct CompletionArgs {
	#[arg(value_enum, value_name = "SHELL")]
	pub shell: clap_complete::Shell,
}

