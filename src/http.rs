use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Method, StatusCode};
use serde_json::Value;
use url::Url;

use crate::error::CliError;

const AUTH_HEADER: &str = "x-ztnet-auth";

#[derive(Debug, Clone)]
pub struct HttpClient {
	base_url: Url,
	token: Option<String>,
	retries: u32,
	dry_run: bool,
	client: reqwest::Client,
}

impl HttpClient {
	pub fn new(
		base_url: &str,
		token: Option<String>,
		timeout: Duration,
		retries: u32,
		dry_run: bool,
	) -> Result<Self, CliError> {
		let base_url = Url::parse(base_url)?;
		let client = reqwest::Client::builder().timeout(timeout).build()?;
		Ok(Self {
			base_url,
			token,
			retries,
			dry_run,
			client,
		})
	}

	pub fn build_url(&self, path: &str) -> Result<Url, CliError> {
		if path.starts_with("http://") || path.starts_with("https://") {
			return Ok(Url::parse(path)?);
		}
		Ok(self.base_url.join(path)?)
	}

	pub async fn request_json(
		&self,
		method: Method,
		path: &str,
		body: Option<Value>,
		headers: HeaderMap,
		include_auth: bool,
	) -> Result<Value, CliError> {
		let url = self.build_url(path)?;
		let body_bytes = match body {
			Some(v) => Some(serde_json::to_vec(&v)?),
			None => None,
		};

		if self.dry_run {
			print_dry_run(&method, &url, include_auth.then(|| self.token.as_deref()).flatten(), &headers, body_bytes.as_deref());
			return Err(CliError::DryRunPrinted);
		}

		let mut backoff = Duration::from_millis(200);
		for attempt in 0..=self.retries {
			let mut request_headers = headers.clone();
			request_headers.insert("accept", HeaderValue::from_static("application/json"));

			if include_auth {
				let token = self.token.as_deref().ok_or(CliError::MissingConfig("token"))?;
				request_headers.insert(
					HeaderName::from_static(AUTH_HEADER),
					HeaderValue::from_str(token)
						.map_err(|_| CliError::InvalidArgument("token contains invalid characters".to_string()))?,
				);
			}

			let mut request = self.client.request(method.clone(), url.clone()).headers(request_headers);
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

fn print_dry_run(
	method: &Method,
	url: &Url,
	token: Option<&str>,
	headers: &HeaderMap,
	body: Option<&[u8]>,
) {
	println!("{method} {url}");

	for (name, value) in headers.iter() {
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

