use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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

pub(crate) fn join_relative_url(base_url: &Url, path: &str) -> Result<Url, CliError> {
	let relative = path.trim().trim_start_matches('/');
	Ok(base_url.join(relative)?)
}

pub(crate) fn normalize_and_join_url(base_url: &mut Url, path: &str) -> Result<Url, CliError> {
	normalize_base_url_for_join(base_url);
	join_relative_url(base_url, path)
}

pub(crate) fn parse_normalize_and_join_url(base: &str, path: &str) -> Result<Url, CliError> {
	let mut base_url = Url::parse(base)?;
	normalize_and_join_url(&mut base_url, path)
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
	join_relative_url(&base.url, path)
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

pub(crate) async fn try_with_base_fallback<T, F, Fut, ShouldTry, OnSwitch>(
	bases: &[BaseCandidate],
	active_base: &AtomicUsize,
	path: &str,
	allow_absolute: bool,
	should_try_host_autofix: ShouldTry,
	mut attempt: F,
	mut on_switch: OnSwitch,
) -> Result<T, CliError>
where
	F: FnMut(Url) -> Fut,
	Fut: Future<Output = Result<T, CliError>>,
	ShouldTry: Fn(&CliError) -> bool,
	OnSwitch: FnMut(usize),
{
	let path = path.trim();
	let is_absolute = allow_absolute && (path.starts_with("http://") || path.starts_with("https://"));

	let base_idx = active_base.load(Ordering::Relaxed);
	let url = build_url_for_base(bases, base_idx, path, allow_absolute)?;
	let result = attempt(url).await;

	if is_absolute || bases.len() < 2 {
		return result;
	}

	match result {
		Ok(value) => Ok(value),
		Err(err) if should_try_host_autofix(&err) => {
			for idx in 0..bases.len() {
				if idx == base_idx {
					continue;
				}

				let url = build_url_for_base(bases, idx, path, allow_absolute)?;
				let alt_result = attempt(url).await;
				if let Ok(value) = alt_result {
					active_base.store(idx, Ordering::Relaxed);
					on_switch(idx);
					return Ok(value);
				}
			}

			Err(err)
		}
		Err(err) => Err(err),
	}
}
