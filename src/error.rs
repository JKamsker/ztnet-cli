use std::io;

use reqwest::StatusCode;
use thiserror::Error;

use crate::config::ConfigError;

#[derive(Debug, Error)]
pub enum CliError {
	#[error(transparent)]
	Config(#[from] ConfigError),

	#[error("missing required configuration: {0}")]
	MissingConfig(&'static str),

	#[error("invalid argument: {0}")]
	InvalidArgument(String),

	#[error("dry-run: request printed")]
	DryRunPrinted,

	#[error("request failed: {0}")]
	Request(#[from] reqwest::Error),

	#[error("http {status}: {message}")]
	HttpStatus {
		status: StatusCode,
		message: String,
		body: Option<String>,
	},

	#[error("rate limited (429) after retries exhausted")]
	RateLimited,

	#[error("I/O error: {0}")]
	Io(#[from] io::Error),

	#[error("failed to parse json: {0}")]
	Json(#[from] serde_json::Error),

	#[error("invalid url: {0}")]
	Url(#[from] url::ParseError),
}

impl CliError {
	pub fn exit_code(&self) -> i32 {
		match self {
			CliError::DryRunPrinted => 0,
			CliError::MissingConfig(_) | CliError::InvalidArgument(_) => 2,
			CliError::RateLimited => 6,
			CliError::HttpStatus { status, .. } => match *status {
				StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => 3,
				StatusCode::NOT_FOUND => 4,
				StatusCode::CONFLICT | StatusCode::UNPROCESSABLE_ENTITY => 5,
				StatusCode::TOO_MANY_REQUESTS => 6,
				_ => 1,
			},
			_ => 1,
		}
	}
}

