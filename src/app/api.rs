use std::io::{self, Write};
use std::path::PathBuf;

use reqwest::Method;
use serde_json::Value;

use crate::cli::{ApiCommand, GlobalOpts};
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::http::{ClientUi, HttpClient};
use crate::output;

use super::common::load_config_store;

pub(super) async fn run(global: &GlobalOpts, command: ApiCommand) -> Result<(), CliError> {
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
		ApiCommand::Request(args) => {
			let method = parse_method(&args.method)?;
			exec_api_request(
				global,
				&effective,
				&client,
				method,
				&args.path,
				args.body,
				args.body_file,
				args.header,
				args.no_auth,
				args.raw,
			)
			.await
		}
		ApiCommand::Get(args) => {
			exec_api_request(
				global,
				&effective,
				&client,
				Method::GET,
				&args.path,
				None,
				None,
				vec![],
				false,
				false,
			)
			.await
		}
		ApiCommand::Post(args) => {
			exec_api_request(
				global,
				&effective,
				&client,
				Method::POST,
				&args.path,
				args.body,
				args.body_file,
				vec![],
				false,
				false,
			)
			.await
		}
		ApiCommand::Delete(args) => {
			exec_api_request(
				global,
				&effective,
				&client,
				Method::DELETE,
				&args.path,
				None,
				None,
				vec![],
				false,
				false,
			)
			.await
		}
	}
}

async fn exec_api_request(
	global: &GlobalOpts,
	effective: &crate::context::EffectiveConfig,
	client: &HttpClient,
	method: Method,
	path: &str,
	body: Option<String>,
	body_file: Option<PathBuf>,
	headers: Vec<String>,
	no_auth: bool,
	raw: bool,
) -> Result<(), CliError> {
	let mut header_map = reqwest::header::HeaderMap::new();
	for raw_header in headers {
		let (k, v) = raw_header.split_once(':').ok_or_else(|| {
			CliError::InvalidArgument(format!("invalid header (expected K:V): {raw_header}"))
		})?;

		let name = reqwest::header::HeaderName::from_bytes(k.trim().as_bytes()).map_err(|_| {
			CliError::InvalidArgument(format!("invalid header name: {}", k.trim()))
		})?;
		let value = reqwest::header::HeaderValue::from_str(v.trim()).map_err(|_| {
			CliError::InvalidArgument(format!("invalid header value for: {}", k.trim()))
		})?;
		header_map.insert(name, value);
	}

	let include_auth = !no_auth && path.trim_start().starts_with("/api/v1");

	let body_value = if let Some(body) = body {
		Some(
			serde_json::from_str::<Value>(&body)
				.map_err(|err| CliError::InvalidArgument(format!("invalid --body json: {err}")))?,
		)
	} else if let Some(path) = body_file {
		let text = std::fs::read_to_string(&path)?;
		Some(serde_json::from_str::<Value>(&text).map_err(|err| {
			CliError::InvalidArgument(format!("invalid --body-file json: {err}"))
		})?)
	} else {
		None
	};

	if raw {
		let body_bytes = body_value
			.as_ref()
			.map(|v| serde_json::to_vec(v))
			.transpose()?;

		let bytes = client
			.request_bytes(
				method,
				path,
				body_bytes,
				header_map,
				include_auth,
				body_value.as_ref().map(|_| "application/json"),
			)
			.await?;

		io::stdout().write_all(&bytes)?;
		return Ok(());
	}

	let response = client
		.request_json(method, path, body_value, header_map, include_auth)
		.await?;

	output::print_value(&response, effective.output, global.no_color)?;
	Ok(())
}

fn parse_method(raw: &str) -> Result<Method, CliError> {
	let raw = raw.trim().to_ascii_uppercase();
	Method::from_bytes(raw.as_bytes())
		.map_err(|_| CliError::InvalidArgument(format!("invalid http method: {raw}")))
}
