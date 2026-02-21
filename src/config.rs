use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::cli::OutputFormat;

#[derive(Debug, Error)]
pub enum ConfigError {
	#[error("failed to determine config directory")]
	NoConfigDir,

	#[error("failed to read config file: {path}")]
	Read {
		path: PathBuf,
		#[source]
		source: io::Error,
	},

	#[error("failed to parse config file: {path}")]
	Parse {
		path: PathBuf,
		#[source]
		source: toml::de::Error,
	},

	#[error("failed to serialize config")]
	Serialize {
		#[source]
		source: toml::ser::Error,
	},

	#[error("failed to write config file: {path}")]
	Write {
		path: PathBuf,
		#[source]
		source: io::Error,
	},

	#[error("invalid output format: {0}")]
	InvalidOutputFormat(String),

	#[error("invalid timeout value: {0}")]
	InvalidTimeout(String),
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
	#[serde(default)]
	pub active_profile: Option<String>,

	#[serde(default)]
	pub profiles: BTreeMap<String, ProfileConfig>,

	#[serde(default)]
	pub host_defaults: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ProfileConfig {
	#[serde(default)]
	pub host: Option<String>,

	#[serde(default)]
	pub token: Option<String>,

	#[serde(default)]
	pub session_cookie: Option<String>,

	#[serde(default)]
	pub device_cookie: Option<String>,

	#[serde(default)]
	pub default_org: Option<String>,

	#[serde(default)]
	pub default_network: Option<String>,

	#[serde(default)]
	pub output: Option<OutputFormat>,

	#[serde(default)]
	pub timeout: Option<String>,

	#[serde(default)]
	pub retries: Option<u32>,
}

impl Config {
	pub fn profile(&self, name: &str) -> ProfileConfig {
		self.profiles.get(name).cloned().unwrap_or_default()
	}

	pub fn profile_mut(&mut self, name: &str) -> &mut ProfileConfig {
		self.profiles.entry(name.to_string()).or_default()
	}
}

pub fn default_config_path() -> Result<PathBuf, ConfigError> {
	let dir = default_config_dir()?;
	Ok(dir.join("config.toml"))
}

fn default_config_dir() -> Result<PathBuf, ConfigError> {
	#[cfg(target_os = "windows")]
	{
		let app_data = env::var_os("APPDATA").ok_or(ConfigError::NoConfigDir)?;
		return Ok(PathBuf::from(app_data).join("ztnet"));
	}

	#[cfg(target_os = "macos")]
	{
		let home = env::var_os("HOME").ok_or(ConfigError::NoConfigDir)?;
		return Ok(PathBuf::from(home)
			.join("Library")
			.join("Application Support")
			.join("ztnet"));
	}

	#[cfg(not(any(target_os = "windows", target_os = "macos")))]
	{
		if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
			return Ok(PathBuf::from(xdg).join("ztnet"));
		}

		let home = env::var_os("HOME").ok_or(ConfigError::NoConfigDir)?;
		Ok(PathBuf::from(home).join(".config").join("ztnet"))
	}
}

pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
	match fs::read_to_string(path) {
		Ok(contents) => toml::from_str(&contents).map_err(|source| ConfigError::Parse {
			path: path.to_path_buf(),
			source,
		}),
		Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
		Err(source) => Err(ConfigError::Read {
			path: path.to_path_buf(),
			source,
		}),
	}
}

pub fn save_config(path: &Path, config: &Config) -> Result<(), ConfigError> {
	let contents = toml::to_string_pretty(config).map_err(|source| ConfigError::Serialize {
		source,
	})?;

	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent).map_err(|source| ConfigError::Write {
			path: parent.to_path_buf(),
			source,
		})?;
	}

	fs::write(path, contents).map_err(|source| ConfigError::Write {
		path: path.to_path_buf(),
		source,
	})
}
