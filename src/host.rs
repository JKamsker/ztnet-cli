use url::Url;

use crate::error::CliError;

pub(crate) fn normalize_host_input(raw: &str) -> Result<String, CliError> {
	let trimmed = raw.trim();
	if trimmed.is_empty() {
		return Err(CliError::InvalidArgument("host cannot be empty".to_string()));
	}

	let with_scheme = if trimmed.contains("://") {
		trimmed.to_string()
	} else {
		let scheme = infer_default_scheme(trimmed);
		format!("{scheme}://{trimmed}")
	};

	let mut url = Url::parse(&with_scheme)
		.map_err(|err| CliError::InvalidArgument(format!("invalid host url: {err}")))?;

	let scheme = url.scheme().to_ascii_lowercase();
	if scheme != "http" && scheme != "https" {
		return Err(CliError::InvalidArgument(format!(
			"invalid host url: unsupported scheme '{scheme}' (expected http or https)"
		)));
	}

	if url.host_str().is_none() {
		return Err(CliError::InvalidArgument(
			"invalid host url: missing hostname".to_string(),
		));
	}

	if !url.username().is_empty() || url.password().is_some() {
		return Err(CliError::InvalidArgument(
			"invalid host url: must not include credentials".to_string(),
		));
	}

	url.set_query(None);
	url.set_fragment(None);

	let mut out = url.to_string();
	while out.ends_with('/') {
		out.pop();
	}
	Ok(out)
}

pub(crate) fn api_base_candidates(base: &str) -> Vec<String> {
	let base = base.trim_end_matches('/');

	let mut out = Vec::with_capacity(2);
	if !base.is_empty() {
		out.push(base.to_string());
	}

	if let Some(stripped) = base.strip_suffix("/api") {
		if !stripped.is_empty() && !out.iter().any(|v| v == stripped) {
			out.push(stripped.to_string());
		}
	} else {
		let candidate = format!("{base}/api");
		if !out.iter().any(|v| v == &candidate) {
			out.push(candidate);
		}
	}

	out
}

fn infer_default_scheme(raw: &str) -> &'static str {
	let before_slash = raw.split('/').next().unwrap_or(raw);

	let host_part = if let Some(rest) = before_slash.strip_prefix('[') {
		if let Some(end) = rest.find(']') {
			&rest[..end]
		} else {
			before_slash
		}
	} else {
		before_slash.split(':').next().unwrap_or(before_slash)
	};

	let host_lower = host_part.to_ascii_lowercase();
	if host_lower == "localhost"
		|| host_lower == "::1"
		|| host_lower.starts_with("127.")
		|| host_lower == "0.0.0.0"
	{
		"http"
	} else {
		"https"
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn normalize_host_input_adds_default_scheme() {
		assert_eq!(
			normalize_host_input("ztnet.example.com/api").unwrap(),
			"https://ztnet.example.com/api"
		);
	}

	#[test]
	fn normalize_host_input_defaults_localhost_to_http() {
		assert_eq!(
			normalize_host_input("localhost:3000").unwrap(),
			"http://localhost:3000"
		);
		assert_eq!(
			normalize_host_input("[::1]:3000").unwrap(),
			"http://[::1]:3000"
		);
	}

	#[test]
	fn normalize_host_input_trims_and_removes_trailing_slash() {
		assert_eq!(
			normalize_host_input(" HTTPS://Example.com/ ").unwrap(),
			"https://example.com"
		);
		assert_eq!(
			normalize_host_input("https://example.com/api/").unwrap(),
			"https://example.com/api"
		);
	}

	#[test]
	fn normalize_host_input_rejects_non_http_schemes() {
		let err = normalize_host_input("ftp://example.com").unwrap_err();
		match err {
			CliError::InvalidArgument(_) => {}
			other => panic!("expected InvalidArgument, got {other:?}"),
		}
	}
}
