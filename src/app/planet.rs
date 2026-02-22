use std::io::{self, Write};
use std::path::PathBuf;

use reqwest::Method;

use crate::cli::{GlobalOpts, PlanetCommand};
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::http::{ClientUi, HttpClient};

use super::common::load_config_store;

pub(super) async fn run(global: &GlobalOpts, command: PlanetCommand) -> Result<(), CliError> {
	let (_config_path, cfg) = load_config_store()?;
	let effective = resolve_effective_config(global, &cfg)?;

	match command {
		PlanetCommand::Download(args) => {
			if args.stdout && args.out.is_some() {
				return Err(CliError::InvalidArgument(
					"cannot combine --stdout with --out".to_string(),
				));
			}

			let out_path = if args.stdout {
				None
			} else {
				Some(args.out.unwrap_or_else(|| PathBuf::from("planet")))
			};

			if let Some(ref out_path) = out_path {
				if out_path.exists() && !args.force {
					return Err(CliError::InvalidArgument(format!(
						"output file already exists: {} (pass --force to overwrite)",
						out_path.display()
					)));
				}
			}

			let client = HttpClient::new(
				&effective.host,
				None,
				effective.timeout,
				effective.retries,
				global.dry_run,
				ClientUi::from_context(global, &effective),
			)?;

			let bytes = client
				.request_bytes(
					Method::GET,
					"/api/planet",
					None,
					Default::default(),
					false,
					None,
				)
				.await?;

			if let Some(out_path) = out_path {
				if let Some(parent) = out_path.parent() {
					std::fs::create_dir_all(parent)?;
				}
				std::fs::write(&out_path, &bytes)?;
				if !global.quiet {
					eprintln!("Wrote {} bytes to {}.", bytes.len(), out_path.display());
				}
				return Ok(());
			}

			io::stdout().write_all(&bytes)?;
			Ok(())
		}
	}
}
