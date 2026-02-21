use std::io::{self, Write};

use comfy_table::{presets, Cell, Table};
use serde_json::Value;

use crate::cli::OutputFormat;
use crate::error::CliError;

pub fn print_value(value: &Value, format: OutputFormat, no_color: bool) -> Result<(), CliError> {
	let mut stdout = io::stdout().lock();
	write_value(&mut stdout, value, format, no_color)?;
	writeln!(&mut stdout)?;
	Ok(())
}

pub fn write_value<W: Write>(
	mut writer: W,
	value: &Value,
	format: OutputFormat,
	no_color: bool,
) -> Result<(), CliError> {
	match format {
		OutputFormat::Json => {
			let pretty = serde_json::to_string_pretty(value)?;
			write!(writer, "{pretty}")?;
		}
		OutputFormat::Yaml => {
			let yaml = serde_yaml::to_string(value)
				.map_err(|err| CliError::InvalidArgument(format!("yaml serialize error: {err}")))?;
			write!(writer, "{yaml}")?;
		}
		OutputFormat::Raw => {
			let compact = serde_json::to_string(value)?;
			write!(writer, "{compact}")?;
		}
		OutputFormat::Table => {
			if !write_table(&mut writer, value, no_color)? {
				let pretty = serde_json::to_string_pretty(value)?;
				write!(writer, "{pretty}")?;
			}
		}
	}

	Ok(())
}

fn write_table<W: Write>(mut writer: W, value: &Value, _no_color: bool) -> Result<bool, CliError> {
	let Some(rows) = value.as_array() else {
		return Ok(false);
	};

	let mut table = Table::new();
	table.load_preset(presets::UTF8_FULL);

	let preferred_columns = [
		"id",
		"name",
		"orgName",
		"nwid",
		"nwname",
		"authorized",
		"memberCount",
		"host",
		"default_profile",
		"profiles",
	];

	let mut columns: Vec<&'static str> = Vec::new();
	for col in preferred_columns {
		if rows.iter().any(|row| row.get(col).is_some()) {
			columns.push(col);
		}
	}
	if columns.is_empty() {
		return Ok(false);
	}

	table.set_header(columns.iter().copied());

	for row in rows {
		let mut cells = Vec::with_capacity(columns.len());
		for col in &columns {
			let text = row.get(*col).map(value_to_cell).unwrap_or_default();
			cells.push(Cell::new(text));
		}
		table.add_row(cells);
	}

	write!(writer, "{table}")?;
	Ok(true)
}

fn value_to_cell(value: &Value) -> String {
	match value {
		Value::Null => String::new(),
		Value::Bool(v) => v.to_string(),
		Value::Number(v) => v.to_string(),
		Value::String(v) => v.clone(),
		_ => serde_json::to_string(value).unwrap_or_default(),
	}
}
