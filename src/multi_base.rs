use std::sync::atomic::{AtomicBool, Ordering};

use reqwest::StatusCode;
use url::Url;

use crate::error::CliError;
use crate::host::{api_base_candidates, normalize_host_input};

#[derive(Debug)]
pub(crate) struct BaseCandidate {
	pub display: String,
	pub url: Url,
}

pub(crate) fn build_base_candidates(base_url: &str) -> Result<Vec<BaseCandidate>, CliError> {
	let base_url = normalize_host_input(base_url)?;
	let candidates = api_base_candidates(&base_url);
	let mut bases = Vec::with_capacity(candidates.len());
	for candidate in candidates {
		let mut url = Url::parse(&candidate)?;
		normalize_base_url_for_join(&mut url);
		bases.push(BaseCandidate {
			display: candidate,
			url,
		});
	}

	if bases.is_empty() {
		return Err(CliError::InvalidArgument("host cannot be empty".to_string()));
	}

	Ok(bases)
}

pub(crate) fn normalize_base_url_for_join(url: &mut Url) {
	url.set_query(None);
	url.set_fragment(None);

	let path = url.path();
	if !path.ends_with('/') {
		let mut new_path = path.to_string();
		new_path.push('/');
		url.set_path(&new_path);
	}
}

pub(crate) fn build_url_for_base(
	bases: &[BaseCandidate],
	base_idx: usize,
	path: &str,
	allow_absolute: bool,
) -> Result<Url, CliError> {
	let path = path.trim();
	if allow_absolute && (path.starts_with("http://") || path.starts_with("https://")) {
		return Ok(Url::parse(path)?);
	}

	let base = bases.get(base_idx).ok_or_else(|| {
		CliError::InvalidArgument("invalid internal host base index".to_string())
	})?;
	let relative = path.trim_start_matches('/');
	Ok(base.url.join(relative)?)
}

pub(crate) fn maybe_warn_host_autofix<F>(
	quiet: bool,
	warned_autofix: &AtomicBool,
	bases: &[BaseCandidate],
	active_idx: usize,
	print_banner: F,
) where
	F: FnOnce(&str, &str),
{
	if quiet {
		return;
	}
	if active_idx == 0 {
		return;
	}
	if warned_autofix.swap(true, Ordering::Relaxed) {
		return;
	}

	let Some(configured) = bases.first().map(|b| b.display.as_str()) else {
		return;
	};
	let Some(using) = bases.get(active_idx).map(|b| b.display.as_str()) else {
		return;
	};

	print_banner(configured, using);
}

pub(crate) fn should_try_host_autofix_basic(err: &CliError) -> bool {
	match err {
		CliError::HttpStatus { status, .. } => {
			matches!(*status, StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED)
		}
		CliError::Request(err) => err.is_decode(),
		_ => false,
	}
}
