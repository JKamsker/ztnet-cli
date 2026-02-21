use std::io::{self, Read, Write};
use std::path::PathBuf;

use serde_json::Value;

use crate::cli::{GlobalOpts, OutputFormat};
use crate::config::{self, Config};
use crate::error::CliError;
use crate::output;

pub(super) fn confirm(global: &GlobalOpts, prompt: &str) -> Result<bool, CliError> {
	if global.dry_run {
		return Ok(true);
	}
	if global.yes {
		return Ok(true);
	}
	if global.quiet {
		return Err(CliError::InvalidArgument(
			"refusing to prompt in --quiet mode (pass --yes)".to_string(),
		));
	}

	eprint!("{prompt}[y/N]: ");
	io::stderr().flush()?;

	let mut input = String::new();
	io::stdin().read_line(&mut input)?;
	let input = input.trim().to_ascii_lowercase();
	Ok(matches!(input.as_str(), "y" | "yes"))
}

pub(super) fn write_text_output(
	out: &str,
	path: Option<&PathBuf>,
	global: &GlobalOpts,
) -> Result<(), CliError> {
	if let Some(path) = path {
		if let Some(parent) = path.parent() {
			std::fs::create_dir_all(parent)?;
		}
		std::fs::write(path, out)?;
		if !global.quiet {
			eprintln!("Wrote {} bytes to {}.", out.as_bytes().len(), path.display());
		}
		return Ok(());
	}

	print!("{out}");
	Ok(())
}

pub(super) fn load_config_store() -> Result<(PathBuf, Config), CliError> {
	let config_path = config::default_config_path()?;
	let cfg = config::load_config(&config_path)?;
	Ok((config_path, cfg))
}

pub(super) fn read_stdin_trimmed() -> Result<String, CliError> {
	let mut input = String::new();
	io::stdin().read_to_string(&mut input)?;
	Ok(input.trim().to_string())
}

pub(super) fn print_human_or_machine(
	value: &Value,
	format: OutputFormat,
	no_color: bool,
) -> Result<(), CliError> {
	if matches!(format, OutputFormat::Table) {
		print_kv(value);
		return Ok(());
	}
	output::print_value(value, format, no_color)
}

pub(super) fn print_kv(value: &Value) {
	let Some(obj) = value.as_object() else {
		println!("{value}");
		return;
	};

	let mut keys: Vec<&String> = obj.keys().collect();
	keys.sort();
	for key in keys {
		let v = &obj[key];
		println!("{key}: {}", render_scalar(v));
	}
}

pub(super) fn render_scalar(value: &Value) -> String {
	match value {
		Value::Null => String::new(),
		Value::Bool(v) => v.to_string(),
		Value::Number(v) => v.to_string(),
		Value::String(v) => v.clone(),
		_ => value.to_string(),
	}
}

pub(super) fn opt_string(value: Option<String>) -> Value {
	value.map(Value::String).unwrap_or(Value::Null)
}

pub(super) fn redact_token(token: &str) -> String {
	const KEEP: usize = 4;
	if token.len() <= KEEP * 2 {
		return "REDACTED".to_string();
	}
	format!("{}�{}", &token[..KEEP], &token[token.len() - KEEP..])
}

