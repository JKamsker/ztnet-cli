use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Method, StatusCode};
use serde_json::Value;
use url::Url;

use crate::error::CliError;
use crate::host::{api_base_candidates, normalize_host_input};

const AUTH_HEADER: &str = "x-ztnet-auth";

#[derive(Debug, Clone, Default)]
pub(crate) struct ClientUi {
	pub quiet: bool,
	pub no_color: bool,
	pub profile: Option<String>,
}

impl ClientUi {
	pub fn new(quiet: bool, no_color: bool, profile: Option<String>) -> Self {
		Self {
			quiet,
			no_color,
			profile,
		}
	}

	fn fix_command(&self, host: &str) -> String {
		match self.profile.as_deref() {
			Some(profile) if profile != "default" => {
				format!("ztnet --profile {profile} config set host {host}")
			}
			_ => format!("ztnet config set host {host}"),
		}
	}
}

#[derive(Debug)]
struct BaseCandidate {
	display: String,
	url: Url,
}

#[derive(Debug)]
pub struct HttpClient {
	bases: Vec<BaseCandidate>,
	active_base: AtomicUsize,
	warned_autofix: AtomicBool,
	token: Option<String>,
	retries: u32,
	dry_run: bool,
	client: reqwest::Client,
	ui: ClientUi,
}

impl HttpClient {
	pub fn new(
		base_url: &str,
		token: Option<String>,
		timeout: Duration,
		retries: u32,
		dry_run: bool,
		ui: ClientUi,
	) -> Result<Self, CliError> {
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

		let client = reqwest::Client::builder().timeout(timeout).build()?;
		Ok(Self {
			bases,
			active_base: AtomicUsize::new(0),
			warned_autofix: AtomicBool::new(false),
			token,
			retries,
			dry_run,
			client,
			ui,
		})
	}

	pub fn build_url(&self, path: &str) -> Result<Url, CliError> {
		let idx = self.active_base.load(Ordering::Relaxed);
		self.build_url_for_base(idx, path)
	}

	fn build_url_for_base(&self, base_idx: usize, path: &str) -> Result<Url, CliError> {
		let path = path.trim();
		if path.starts_with("http://") || path.starts_with("https://") {
			return Ok(Url::parse(path)?);
		}

		let relative = path.strip_prefix('/').unwrap_or(path);
		let base = self.bases.get(base_idx).ok_or_else(|| {
			CliError::InvalidArgument("invalid internal host base index".to_string())
		})?;
		Ok(base.url.join(relative)?)
	}

	fn maybe_warn_host_autofix(&self, active_idx: usize) {
		if self.ui.quiet {
			return;
		}
		if active_idx == 0 {
			return;
		}
		if self.warned_autofix.swap(true, Ordering::Relaxed) {
			return;
		}

		let Some(configured) = self.bases.first().map(|b| b.display.as_str()) else {
			return;
		};
		let Some(using) = self.bases.get(active_idx).map(|b| b.display.as_str()) else {
			return;
		};

		print_host_autofix_banner(&self.ui, configured, using);
	}

	pub async fn request_json(
		&self,
		method: Method,
		path: &str,
		body: Option<Value>,
		headers: HeaderMap,
		include_auth: bool,
	) -> Result<Value, CliError> {
		let path = path.trim();
		let is_absolute = path.starts_with("http://") || path.starts_with("https://");

		let body_bytes = match body {
			Some(v) => Some(serde_json::to_vec(&v)?),
			None => None,
		};

		let base_idx = self.active_base.load(Ordering::Relaxed);
		let url = self.build_url_for_base(base_idx, path)?;

		if self.dry_run {
			print_dry_run(
				&method,
				&url,
				include_auth.then(|| self.token.as_deref()).flatten(),
				&headers,
				body_bytes.as_deref(),
			);
			return Err(CliError::DryRunPrinted);
		}

		let result = self
			.request_json_with_url(method.clone(), url, body_bytes.clone(), &headers, include_auth)
			.await;

		if is_absolute || self.bases.len() < 2 {
			return result;
		}

		match result {
			Ok(value) => Ok(value),
			Err(err) if should_try_host_autofix(&err) => {
				for idx in 0..self.bases.len() {
					if idx == base_idx {
						continue;
					}

					let url = self.build_url_for_base(idx, path)?;
					let attempt = self
						.request_json_with_url(
							method.clone(),
							url,
							body_bytes.clone(),
							&headers,
							include_auth,
						)
						.await;
					if let Ok(value) = attempt {
						self.active_base.store(idx, Ordering::Relaxed);
						self.maybe_warn_host_autofix(idx);
						return Ok(value);
					}
				}

				Err(err)
			}
			Err(err) => Err(err),
		}
	}

	pub async fn request_bytes(
		&self,
		method: Method,
		path: &str,
		body: Option<Vec<u8>>,
		headers: HeaderMap,
		include_auth: bool,
		content_type: Option<&str>,
	) -> Result<Vec<u8>, CliError> {
		let path = path.trim();
		let is_absolute = path.starts_with("http://") || path.starts_with("https://");

		let base_idx = self.active_base.load(Ordering::Relaxed);
		let url = self.build_url_for_base(base_idx, path)?;

		if self.dry_run {
			print_dry_run(
				&method,
				&url,
				include_auth.then(|| self.token.as_deref()).flatten(),
				&headers,
				body.as_deref(),
			);
			return Err(CliError::DryRunPrinted);
		}

		let result = self
			.request_bytes_with_url(
				method.clone(),
				url,
				body.clone(),
				&headers,
				include_auth,
				content_type,
			)
			.await;

		if is_absolute || self.bases.len() < 2 {
			return result;
		}

		match result {
			Ok(bytes) => Ok(bytes),
			Err(err) if should_try_host_autofix(&err) => {
				for idx in 0..self.bases.len() {
					if idx == base_idx {
						continue;
					}

					let url = self.build_url_for_base(idx, path)?;
					let attempt = self
						.request_bytes_with_url(
							method.clone(),
							url,
							body.clone(),
							&headers,
							include_auth,
							content_type,
						)
						.await;
					if let Ok(bytes) = attempt {
						self.active_base.store(idx, Ordering::Relaxed);
						self.maybe_warn_host_autofix(idx);
						return Ok(bytes);
					}
				}

				Err(err)
			}
			Err(err) => Err(err),
		}
	}

	async fn request_json_with_url(
		&self,
		method: Method,
		url: Url,
		body_bytes: Option<Vec<u8>>,
		headers: &HeaderMap,
		include_auth: bool,
	) -> Result<Value, CliError> {
		let mut backoff = Duration::from_millis(200);
		for attempt in 0..=self.retries {
			let mut request_headers = headers.clone();
			request_headers.insert("accept", HeaderValue::from_static("application/json"));

			if include_auth {
				let token = self.token.as_deref().ok_or(CliError::MissingConfig("token"))?;
				request_headers.insert(
					HeaderName::from_static(AUTH_HEADER),
					HeaderValue::from_str(token).map_err(|_| {
						CliError::InvalidArgument("token contains invalid characters".to_string())
					})?,
				);
			}

			let mut request = self
				.client
				.request(method.clone(), url.clone())
				.headers(request_headers);
			if let Some(bytes) = body_bytes.clone() {
				request = request
					.header("content-type", "application/json")
					.body(bytes);
			}

			match request.send().await {
				Ok(resp) => {
					let status = resp.status();
					if status.is_success() {
						return Ok(resp.json::<Value>().await?);
					}

					if should_retry_status(status) && attempt < self.retries {
						if status == StatusCode::TOO_MANY_REQUESTS {
							let retry_after = parse_retry_after(&resp);
							tokio::time::sleep(retry_after.unwrap_or(backoff)).await;
						} else {
							tokio::time::sleep(backoff).await;
						}
						backoff = (backoff * 2).min(Duration::from_secs(5));
						continue;
					}

					if status == StatusCode::TOO_MANY_REQUESTS {
						return Err(CliError::RateLimited);
					}

					let body = resp.text().await.ok();
					return Err(CliError::HttpStatus {
						status,
						message: "request failed".to_string(),
						body,
					});
				}
				Err(err) => {
					if attempt < self.retries && should_retry_error(&err) {
						tokio::time::sleep(backoff).await;
						backoff = (backoff * 2).min(Duration::from_secs(5));
						continue;
					}
					return Err(CliError::Request(err));
				}
			}
		}

		Err(CliError::RateLimited)
	}

	async fn request_bytes_with_url(
		&self,
		method: Method,
		url: Url,
		body: Option<Vec<u8>>,
		headers: &HeaderMap,
		include_auth: bool,
		content_type: Option<&str>,
	) -> Result<Vec<u8>, CliError> {
		let mut backoff = Duration::from_millis(200);
		for attempt in 0..=self.retries {
			let mut request_headers = headers.clone();
			request_headers.insert("accept", HeaderValue::from_static("*/*"));

			if include_auth {
				let token = self.token.as_deref().ok_or(CliError::MissingConfig("token"))?;
				request_headers.insert(
					HeaderName::from_static(AUTH_HEADER),
					HeaderValue::from_str(token).map_err(|_| {
						CliError::InvalidArgument("token contains invalid characters".to_string())
					})?,
				);
			}

			let mut request = self
				.client
				.request(method.clone(), url.clone())
				.headers(request_headers);
			if let Some(bytes) = body.clone() {
				if let Some(content_type) = content_type {
					request = request.header("content-type", content_type);
				}
				request = request.body(bytes);
			}

			match request.send().await {
				Ok(resp) => {
					let status = resp.status();
					if status.is_success() {
						return Ok(resp.bytes().await?.to_vec());
					}

					if should_retry_status(status) && attempt < self.retries {
						if status == StatusCode::TOO_MANY_REQUESTS {
							let retry_after = parse_retry_after(&resp);
							tokio::time::sleep(retry_after.unwrap_or(backoff)).await;
						} else {
							tokio::time::sleep(backoff).await;
						}
						backoff = (backoff * 2).min(Duration::from_secs(5));
						continue;
					}

					if status == StatusCode::TOO_MANY_REQUESTS {
						return Err(CliError::RateLimited);
					}

					let body = resp.text().await.ok();
					return Err(CliError::HttpStatus {
						status,
						message: "request failed".to_string(),
						body,
					});
				}
				Err(err) => {
					if attempt < self.retries && should_retry_error(&err) {
						tokio::time::sleep(backoff).await;
						backoff = (backoff * 2).min(Duration::from_secs(5));
						continue;
					}
					return Err(CliError::Request(err));
				}
			}
		}

		Err(CliError::RateLimited)
	}
}

fn normalize_base_url_for_join(url: &mut Url) {
	url.set_query(None);
	url.set_fragment(None);

	let path = url.path();
	if !path.ends_with('/') {
		let mut new_path = path.to_string();
		new_path.push('/');
		url.set_path(&new_path);
	}
}

fn should_try_host_autofix(err: &CliError) -> bool {
	match err {
		CliError::HttpStatus { status, .. } => {
			matches!(*status, StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED)
		}
		CliError::Request(err) => err.is_decode(),
		_ => false,
	}
}

pub(crate) fn print_host_autofix_banner(ui: &ClientUi, configured: &str, using: &str) {
	let fix = ui.fix_command(using);

	if ui.no_color {
		eprintln!("==================== HOST AUTO-FIX ====================");
		eprintln!("Configured: {configured}");
		eprintln!("Using:      {using}");
		eprintln!("Fix:        {fix}");
		eprintln!("======================================================");
		return;
	}

	let yellow = "\x1b[33m";
	let bold = "\x1b[1m";
	let reset = "\x1b[0m";
	eprintln!("{yellow}{bold}==================== HOST AUTO-FIX ===================={reset}");
	eprintln!("{yellow}{bold}Configured:{reset} {configured}");
	eprintln!("{yellow}{bold}Using:     {reset} {using}");
	eprintln!("{yellow}{bold}Fix:       {reset} {fix}");
	eprintln!("{yellow}{bold}======================================================{reset}");
}

fn should_retry_status(status: StatusCode) -> bool {
	status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn should_retry_error(err: &reqwest::Error) -> bool {
	err.is_timeout() || err.is_connect() || err.is_request()
}

fn parse_retry_after(resp: &reqwest::Response) -> Option<Duration> {
	let value = resp.headers().get("retry-after")?.to_str().ok()?;
	let secs = value.trim().parse::<u64>().ok()?;
	Some(Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn build_url_preserves_base_path_prefix() {
		let client = HttpClient::new(
			"https://example.com/api",
			None,
			Duration::from_secs(1),
			0,
			true,
			ClientUi::default(),
		)
		.unwrap();

		let url = client.build_url("/api/v1/network").unwrap();
		assert_eq!(url.as_str(), "https://example.com/api/api/v1/network");
	}

	#[test]
	fn build_url_works_without_path_prefix() {
		let client = HttpClient::new(
			"https://example.com",
			None,
			Duration::from_secs(1),
			0,
			true,
			ClientUi::default(),
		)
		.unwrap();
		let url = client.build_url("/api/v1/network").unwrap();
		assert_eq!(url.as_str(), "https://example.com/api/v1/network");
	}

	#[test]
	fn build_url_allows_absolute_urls() {
		let client = HttpClient::new(
			"https://example.com",
			None,
			Duration::from_secs(1),
			0,
			true,
			ClientUi::default(),
		)
		.unwrap();
		let url = client.build_url("https://other.example.com/x").unwrap();
		assert_eq!(url.as_str(), "https://other.example.com/x");
	}
}

fn print_dry_run(
	method: &Method,
	url: &Url,
	token: Option<&str>,
	headers: &HeaderMap,
	body: Option<&[u8]>,
) {
	println!("{method} {url}");

	for (name, value) in headers.iter() {
		if name.as_str().eq_ignore_ascii_case("cookie") {
			println!("{name}: REDACTED");
			continue;
		}
		if let Ok(value) = value.to_str() {
			println!("{name}: {value}");
		}
	}

	if let Some(token) = token {
		println!("{AUTH_HEADER}: {}", redact_token(token));
	}

	if let Some(body) = body {
		if let Ok(json) = serde_json::from_slice::<Value>(body) {
			if let Ok(pretty) = serde_json::to_string_pretty(&json) {
				println!();
				println!("{pretty}");
				return;
			}
		}

		if let Ok(text) = std::str::from_utf8(body) {
			println!();
			println!("{text}");
		}
	}
}

fn redact_token(token: &str) -> String {
	const KEEP: usize = 4;
	if token.len() <= KEEP * 2 {
		return "REDACTED".to_string();
	}
	format!(
		"{}…{}",
		&token[..KEEP],
		&token[token.len() - KEEP..]
	)
}
