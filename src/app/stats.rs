use reqwest::Method;

use crate::cli::{GlobalOpts, StatsCommand};
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::http::{ClientUi, HttpClient};

use super::common::{load_config_store, print_human_or_machine};

pub(super) async fn run(global: &GlobalOpts, command: StatsCommand) -> Result<(), CliError> {
	let (_config_path, cfg) = load_config_store()?;
	let effective = resolve_effective_config(global, &cfg)?;

	let client = HttpClient::new(
		&effective.host,
		effective.token.clone(),
		effective.timeout,
		effective.retries,
		global.dry_run,
		ClientUi::new(global.quiet, global.no_color, Some(effective.profile.clone())),
	)?;

	match command {
		StatsCommand::Get => {
			let response = client
				.request_json(Method::GET, "/api/v1/stats", None, Default::default(), true)
				.await?;
			print_human_or_machine(&response, effective.output, global.no_color)?;
			Ok(())
		}
	}
}
