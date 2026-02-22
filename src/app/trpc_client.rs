use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Method, StatusCode};
use serde_json::{json, Value};
use url::Url;

use crate::context::EffectiveConfig;
use crate::error::CliError;
use crate::host::{api_base_candidates, normalize_host_input};
use crate::http::{print_host_autofix_banner, ClientUi};

#[derive(Debug)]
struct BaseCandidate {
	display: String,
	url: Url,
}

#[derive(Debug)]
pub(super) struct TrpcClient {
	bases: Vec<BaseCandidate>,
	active_base: AtomicUsize,
	warned_autofix: AtomicBool,
	retries: u32,
	dry_run: bool,
	client: reqwest::Client,
	cookie: Option<String>,
	ui: ClientUi,
}

impl TrpcClient {
	pub(super) fn new(
		base_url: &str,
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
			retries,
			dry_run,
			client,
			cookie: None,
			ui,
		})
	}

	pub(super) fn with_cookie(mut self, cookie: Option<String>) -> Self {
		self.cookie = cookie;
		self
	}

	pub(super) async fn call(&self, procedure: &str, input: Value) -> Result<Value, CliError> {
		let path = format!("api/trpc/{}?batch=1", procedure.trim());

		let body = json!({ "0": { "json": input } });
		let body_bytes = serde_json::to_vec(&body)?;

		let mut headers = HeaderMap::new();
		headers.insert("accept", HeaderValue::from_static("application/json"));
		headers.insert("content-type", HeaderValue::from_static("application/json"));

		if let Some(ref cookie) = self.cookie {
			headers.insert(
				reqwest::header::COOKIE,
				HeaderValue::from_str(cookie).map_err(|_| {
					CliError::InvalidArgument("cookie contains invalid characters".to_string())
				})?,
			);
		}

		let base_idx = self.active_base.load(Ordering::Relaxed);
		let url = self.build_url_for_base(base_idx, &path)?;

		if self.dry_run {
			print_dry_run(&Method::POST, &url, &headers, &body);
			return Err(CliError::DryRunPrinted);
		}

		let result = self
			.call_with_url(url, &headers, &body_bytes)
			.await;

		if self.bases.len() < 2 {
			return result;
		}

		match result {
			Ok(value) => Ok(value),
			Err(err) if should_try_host_autofix(&err) => {
				for idx in 0..self.bases.len() {
					if idx == base_idx {
						continue;
					}

					let url = self.build_url_for_base(idx, &path)?;
					let attempt = self.call_with_url(url, &headers, &body_bytes).await;
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

	fn build_url_for_base(&self, base_idx: usize, path: &str) -> Result<Url, CliError> {
		let base = self.bases.get(base_idx).ok_or_else(|| {
			CliError::InvalidArgument("invalid internal host base index".to_string())
		})?;
		let relative = path.trim().trim_start_matches('/');
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

	async fn call_with_url(
		&self,
		url: Url,
		headers: &HeaderMap,
		body_bytes: &[u8],
	) -> Result<Value, CliError> {
		let mut backoff = Duration::from_millis(200);
		for attempt in 0..=self.retries {
			let request = self
				.client
				.request(Method::POST, url.clone())
				.headers(headers.clone())
				.body(body_bytes.to_vec());

			match request.send().await {
				Ok(resp) => {
					let status = resp.status();
					let retry_after = resp
						.headers()
						.get("retry-after")
						.and_then(|v| v.to_str().ok())
						.and_then(|s| s.trim().parse::<u64>().ok())
						.map(Duration::from_secs);
					let bytes = resp.bytes().await?.to_vec();

					if should_retry_status(status) && attempt < self.retries {
						if status == StatusCode::TOO_MANY_REQUESTS {
							tokio::time::sleep(retry_after.unwrap_or(backoff)).await;
						} else {
							tokio::time::sleep(backoff).await;
						}
						backoff = (backoff * 2).min(Duration::from_secs(5));
						continue;
					}

					return parse_trpc_http_response(status, &bytes);
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
		CliError::HttpStatus { status, message, .. } => {
			matches!(*status, StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED)
				|| message == "invalid json response"
		}
		CliError::Request(err) => err.is_decode(),
		_ => false,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn trpc_join_preserves_base_path_prefix() {
		let client = TrpcClient::new(
			"https://example.com/api",
			Duration::from_secs(1),
			0,
			true,
			ClientUi::default(),
		)
		.unwrap();

		let url = client.build_url_for_base(0, "api/trpc/foo?batch=1").unwrap();
		assert_eq!(url.as_str(), "https://example.com/api/api/trpc/foo?batch=1");
	}
}

pub(super) fn cookie_from_effective(effective: &EffectiveConfig) -> Option<String> {
	let session = effective.session_cookie.as_deref()?.trim();
	if session.is_empty() {
		return None;
	}

	let mut parts = vec![
		format!("next-auth.session-token={session}"),
		format!("__Secure-next-auth.session-token={session}"),
	];

	if let Some(device) = effective.device_cookie.as_deref() {
		let device = device.trim();
		if !device.is_empty() {
			parts.push(format!("next-auth.did-token={device}"));
		}
	}

	Some(parts.join("; "))
}

pub(super) fn require_cookie_from_effective(effective: &EffectiveConfig) -> Result<String, CliError> {
	cookie_from_effective(effective).ok_or(CliError::SessionRequired)
}

fn parse_trpc_http_response(status: StatusCode, bytes: &[u8]) -> Result<Value, CliError> {
	if status == StatusCode::UNAUTHORIZED {
		return Err(CliError::SessionRequired);
	}

	let parsed = serde_json::from_slice::<Value>(bytes);

	let value = match parsed {
		Ok(v) => v,
		Err(_) => {
			let body = String::from_utf8_lossy(bytes).to_string();
			return Err(CliError::HttpStatus {
				status,
				message: "invalid json response".to_string(),
				body: Some(body),
			});
		}
	};

	parse_trpc_envelope(status, value)
}

fn parse_trpc_envelope(http_status: StatusCode, value: Value) -> Result<Value, CliError> {
	let item = match value {
		Value::Array(mut items) => items
			.drain(..)
			.next()
			.ok_or_else(|| CliError::HttpStatus {
				status: http_status,
				message: "empty tRPC response".to_string(),
				body: None,
			})?,
		other => other,
	};

	let Some(obj) = item.as_object() else {
		return Ok(item);
	};

	if let Some(err) = obj.get("error") {
		let message = err
			.get("message")
			.and_then(|v| v.as_str())
			.unwrap_or("tRPC error")
			.to_string();

		let code = err
			.get("data")
			.and_then(|d| d.get("code"))
			.and_then(|v| v.as_str())
			.unwrap_or("");

		let http_status = err
			.get("data")
			.and_then(|d| d.get("httpStatus"))
			.and_then(|v| v.as_u64())
			.and_then(|n| StatusCode::from_u16(n as u16).ok())
			.unwrap_or(http_status);

		if code == "UNAUTHORIZED" || http_status == StatusCode::UNAUTHORIZED {
			return Err(CliError::SessionRequired);
		}

		return Err(CliError::HttpStatus {
			status: http_status,
			message,
			body: Some(err.to_string()),
		});
	}

	let Some(result) = obj.get("result") else {
		return Ok(Value::Object(obj.clone()));
	};

	let data = result.get("data").unwrap_or(&Value::Null);
	if let Some(json) = data.get("json") {
		return Ok(json.clone());
	}

	Ok(data.clone())
}

fn should_retry_status(status: StatusCode) -> bool {
	status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn should_retry_error(err: &reqwest::Error) -> bool {
	err.is_timeout() || err.is_connect() || err.is_request()
}

fn print_dry_run(method: &Method, url: &Url, headers: &HeaderMap, body: &Value) {
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

	if let Ok(pretty) = serde_json::to_string_pretty(body) {
		println!();
		println!("{pretty}");
	}
}
