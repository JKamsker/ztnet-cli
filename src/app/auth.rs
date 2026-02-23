use std::collections::{BTreeMap, BTreeSet};
use std::env;

use reqwest::Method;
use serde_json::json;
use url::Url;

use crate::cli::{AuthCommand, GlobalOpts, OutputFormat};
use crate::config;
use crate::context::{canonical_host_key, canonical_host_key_opt};
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::host::normalize_host_input;
use crate::http::{ClientUi, HttpClient};
use crate::output;

use super::common::{load_config_store, print_human_or_machine, read_stdin_trimmed, redact_token};

pub(super) async fn run(global: &GlobalOpts, command: AuthCommand) -> Result<(), CliError> {
	let (config_path, mut cfg) = load_config_store()?;
	let effective = resolve_effective_config(global, &cfg)?;

	match command {
		AuthCommand::SetToken(args) => {
			if args.stdin && args.token.is_some() {
				return Err(CliError::InvalidArgument(
					"cannot combine --stdin with a positional TOKEN".to_string(),
				));
			}

			let profile = args.profile.unwrap_or_else(|| effective.profile.clone());
			let token = if args.stdin {
				read_stdin_trimmed()?
			} else {
				args.token.ok_or_else(|| {
					CliError::InvalidArgument("missing TOKEN (or pass --stdin)".to_string())
				})?
			};

			if token.is_empty() {
				return Err(CliError::InvalidArgument("token cannot be empty".to_string()));
			}

			let explicit_host = explicit_host_override(global);
			let profile_host = cfg.profile(&profile).host;

			let explicit_host = explicit_host
				.as_deref()
				.map(normalize_host_input)
				.transpose()?;
			let profile_host = non_empty(profile_host)
				.as_deref()
				.map(normalize_host_input)
				.transpose()?;

			if let (Some(explicit), Some(from_profile)) = (&explicit_host, &profile_host) {
				if canonical_host_key(explicit)? != canonical_host_key(from_profile)? {
					return Err(CliError::InvalidArgument(format!(
						"profile '{profile}' is configured for '{from_profile}', but the target host is '{explicit}'",
					)));
				}
			}

			let host_value = explicit_host.or(profile_host).ok_or_else(|| {
				CliError::InvalidArgument(
					"host is required for auth set-token (set profiles.<name>.host, pass --host, or set ZTNET_HOST)"
						.to_string(),
				)
			})?;

			if !args.no_validate && !global.dry_run {
				let client = HttpClient::new(
					&host_value,
					Some(token.clone()),
					effective.timeout,
					effective.retries,
					global.dry_run,
					ClientUi::new(global.quiet, global.no_color, Some(profile.clone())),
				)?;

				let result = client
					.request_json(Method::GET, "/api/v1/network", None, Default::default(), true)
					.await;

				match result {
					Ok(_) => {}
					Err(CliError::HttpStatus { status, .. })
						if matches!(
							status,
							reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN
						) =>
					{
						return Err(CliError::InvalidArgument(format!(
							"token rejected by server ({status})"
						)));
					}
					Err(err) => return Err(err),
				}
			}

			let host_key = canonical_host_key(&host_value)?;

			let profile_cfg = cfg.profile_mut(&profile);
			if non_empty(profile_cfg.host.clone()).is_none() {
				profile_cfg.host = Some(host_value);
			}
			profile_cfg.token = Some(token);

			if cfg.host_defaults.get(&host_key).is_none() {
				cfg.host_defaults.insert(host_key, profile.clone());
			}
			config::save_config(&config_path, &cfg)?;

			if !global.quiet {
				eprintln!("Token saved to profile '{profile}'.");
			}
			Ok(())
		}
		AuthCommand::UnsetToken(args) => {
			let profile = args.profile.unwrap_or_else(|| effective.profile.clone());
			cfg.profile_mut(&profile).token = None;
			config::save_config(&config_path, &cfg)?;

			if !global.quiet {
				eprintln!("Token removed from profile '{profile}'.");
			}
			Ok(())
		}
		AuthCommand::Login(args) => {
			let profile = args.profile.unwrap_or_else(|| effective.profile.clone());
			let email = args
				.email
				.clone()
				.filter(|value| !value.trim().is_empty())
				.ok_or_else(|| {
					CliError::InvalidArgument(
						"missing --email (or environment variable ZTNET_EMAIL)".to_string(),
					)
				})?;

			let explicit_host = explicit_host_override(global);
			let profile_host = cfg.profile(&profile).host;

			let explicit_host = explicit_host
				.as_deref()
				.map(normalize_host_input)
				.transpose()?;
			let profile_host = non_empty(profile_host)
				.as_deref()
				.map(normalize_host_input)
				.transpose()?;

			if let (Some(explicit), Some(from_profile)) = (&explicit_host, &profile_host) {
				if canonical_host_key(explicit)? != canonical_host_key(from_profile)? {
					return Err(CliError::InvalidArgument(format!(
						"profile '{profile}' is configured for '{from_profile}', but the target host is '{explicit}'",
					)));
				}
			}

			let host_value = explicit_host.clone().or(profile_host).ok_or_else(|| {
				CliError::InvalidArgument(
					"host is required for auth login (set profiles.<name>.host, pass --host, or set ZTNET_HOST)"
						.to_string(),
				)
			})?;

			if args.password_stdin && args.password.is_some() {
				return Err(CliError::InvalidArgument(
					"cannot combine --password-stdin with --password".to_string(),
				));
			}

			let password = if args.password_stdin {
				read_stdin_trimmed()?
			} else {
				args.password
					.clone()
					.filter(|value| !value.trim().is_empty())
					.ok_or_else(|| {
						CliError::InvalidArgument(
							"missing --password (or ZTNET_PASSWORD or --password-stdin)".to_string(),
						)
					})?
			};

			if password.trim().is_empty() {
				return Err(CliError::InvalidArgument("password cannot be empty".to_string()));
			}

			if global.dry_run {
				let base = host_value.trim_end_matches('/');
				println!("POST {base}/api/auth/callback/credentials");
				println!("content-type: application/x-www-form-urlencoded");
				println!("(credentials omitted)");
				return Err(CliError::DryRunPrinted);
			}

			let base = host_value.trim_end_matches('/');

			let client = reqwest::Client::builder()
				.timeout(effective.timeout)
				.redirect(reqwest::redirect::Policy::none())
				.build()?;

			let user_agent = format!("ztnet-cli/{}", env!("CARGO_PKG_VERSION"));
			let mut totp = args.totp.clone();
			loop {
				let (csrf_token, csrf_cookie_header) =
					fetch_nextauth_csrf(&client, base, &user_agent).await?;
				let response = nextauth_credentials_login(
					&client,
					base,
					&csrf_token,
					&csrf_cookie_header,
					&email,
					&password,
					&user_agent,
					totp.as_deref(),
				)
				.await?;

				if response.ok {
					let session = response.session_cookie.ok_or_else(|| {
						CliError::HttpStatus {
							status: reqwest::StatusCode::UNAUTHORIZED,
							message: "login succeeded but server did not set a session cookie".to_string(),
							body: None,
						}
					})?;

					let profile_cfg = cfg.profile_mut(&profile);
					if non_empty(profile_cfg.host.clone()).is_none() {
						profile_cfg.host = Some(host_value.to_string());
					}
					profile_cfg.session_cookie = Some(session);
					profile_cfg.device_cookie = response.device_cookie;

					let host_key = canonical_host_key(&host_value)?;
					if cfg.host_defaults.get(&host_key).is_none() {
						cfg.host_defaults.insert(host_key, profile.clone());
					}

					config::save_config(&config_path, &cfg)?;

					if !global.quiet {
						eprintln!("Session saved to profile '{profile}'.");
					}
					return Ok(());
				}

				if response.error.as_deref() == Some("second-factor-required") {
					if totp.is_some() {
						return Err(auth_login_error("two-factor code required"));
					}
					if args.password_stdin {
						return Err(CliError::InvalidArgument(
							"two-factor code required (pass --totp when using --password-stdin)".to_string(),
						));
					}
					if global.quiet {
						return Err(CliError::InvalidArgument(
							"two-factor code required (pass --totp)".to_string(),
						));
					}

					eprint!("Two-factor code: ");
					std::io::Write::flush(&mut std::io::stderr())?;
					let mut code = String::new();
					std::io::stdin().read_line(&mut code)?;
					let code = code.trim().to_string();
					if code.is_empty() {
						return Err(CliError::InvalidArgument("totp code cannot be empty".to_string()));
					}
					totp = Some(code);
					continue;
				}

				let message = match response.error.as_deref() {
					Some("incorrect-username-password") => "invalid email or password",
					Some("incorrect-two-factor-code") => "incorrect two-factor code",
					Some("account-expired") => "account expired",
					Some(err) => err,
					None => "login failed",
				};

				return Err(auth_login_error(message));
			}
		}
		AuthCommand::Logout(args) => {
			let profile = args.profile.unwrap_or_else(|| effective.profile.clone());
			let profile_cfg = cfg.profile_mut(&profile);
			profile_cfg.session_cookie = None;
			profile_cfg.device_cookie = None;
			config::save_config(&config_path, &cfg)?;

			if !global.quiet {
				eprintln!("Session cleared from profile '{profile}'.");
			}
			Ok(())
		}
		AuthCommand::Show => {
			let value = json!({
				"profile": effective.profile,
				"host": effective.host,
				"token": effective.token.as_deref().map(redact_token),
				"session": if effective.session_cookie.is_some() { "active" } else { "none" },
				"device": if effective.device_cookie.is_some() { "present" } else { "none" },
				"org": effective.org,
				"network": effective.network,
				"output": effective.output.to_string(),
				"timeout": humantime::format_duration(effective.timeout).to_string(),
				"retries": effective.retries,
			});
			print_human_or_machine(&value, effective.output, global.no_color)?;
			Ok(())
		}
		AuthCommand::Test(args) => {
			let path = if args.org.is_some() { "/api/v1/org" } else { "/api/v1/network" };

			let client = HttpClient::new(
				&effective.host,
				effective.token.clone(),
				effective.timeout,
				effective.retries,
				global.dry_run,
				ClientUi::from_context(global, &effective),
			)?;

			let response = client
				.request_json(Method::GET, path, None, Default::default(), true)
				.await?;

			if matches!(effective.output, OutputFormat::Table) {
				println!("OK");
				return Ok(());
			}

			output::print_value(&response, effective.output, global.no_color)?;
			Ok(())
		}
		AuthCommand::Profiles { command } => match command {
			crate::cli::AuthProfilesCommand::List => {
				let active = cfg.active_profile.clone();
				let profiles: Vec<String> = cfg.profiles.keys().cloned().collect();
				let value = json!({ "active_profile": active, "profiles": profiles });
				print_human_or_machine(&value, effective.output, global.no_color)?;
				Ok(())
			}
			crate::cli::AuthProfilesCommand::Use(args) => {
				cfg.active_profile = Some(args.name.clone());
				cfg.profile_mut(&args.name);
				config::save_config(&config_path, &cfg)?;

				if !global.quiet {
					eprintln!("Active profile set to '{}'.", args.name);
				}
				Ok(())
			}
		},
		AuthCommand::Hosts { command } => match command {
			crate::cli::AuthHostsCommand::List => auth_hosts_list(&cfg, effective.output, global),
			crate::cli::AuthHostsCommand::SetDefault(args) => {
				auth_hosts_set_default(global, &config_path, &mut cfg, &effective, args)
			}
			crate::cli::AuthHostsCommand::UnsetDefault(args) => {
				auth_hosts_unset_default(global, &config_path, &mut cfg, &effective, args)
			}
		},
	}
}

fn auth_login_error(message: &str) -> CliError {
	CliError::HttpStatus {
		status: reqwest::StatusCode::UNAUTHORIZED,
		message: message.to_string(),
		body: None,
	}
}

struct LoginResponse {
	ok: bool,
	error: Option<String>,
	session_cookie: Option<String>,
	device_cookie: Option<String>,
}

async fn fetch_nextauth_csrf(
	client: &reqwest::Client,
	base: &str,
	user_agent: &str,
) -> Result<(String, String), CliError> {
	let auth_base = auth_root_base(base);
	let mut url = Url::parse(&format!("{auth_base}/api/auth/csrf/"))?;

	let mut cookies: BTreeMap<String, String> = BTreeMap::new();
	for _ in 0..8 {
		let cookie_header = cookie_header_from_pairs(&cookies);
		let mut request = client
			.get(url.clone())
			.header("accept", "application/json")
			.header("user-agent", user_agent);
		if !cookie_header.is_empty() {
			request = request.header("cookie", cookie_header);
		}

		let resp = request.send().await?;
		let status = resp.status();
		let set_cookies = collect_set_cookie(&resp);
		merge_set_cookie_pairs(&mut cookies, &set_cookies);

		if status.is_redirection() {
			let location = resp
				.headers()
				.get(reqwest::header::LOCATION)
				.and_then(|v| v.to_str().ok())
				.unwrap_or("")
				.trim()
				.to_string();
			if location.is_empty() {
				return Err(CliError::HttpStatus {
					status,
					message: "csrf request redirected without a location header".to_string(),
					body: None,
				});
			}

			url = resolve_redirect_url(&url, &location)?;
			continue;
		}

		let body = resp.text().await?;
		if !status.is_success() {
			return Err(CliError::HttpStatus {
				status,
				message: "failed to obtain csrfToken from server".to_string(),
				body: Some(body),
			});
		}

		let value = match serde_json::from_str::<serde_json::Value>(&body) {
			Ok(value) => value,
			Err(err) => {
				return Err(CliError::HttpStatus {
					status: reqwest::StatusCode::BAD_GATEWAY,
					message: format!("failed to parse csrf response: {err}"),
					body: Some(body),
				});
			}
		};

	let csrf = value
		.get("csrfToken")
		.and_then(|v| v.as_str())
		.ok_or_else(|| {
			CliError::HttpStatus {
				status: reqwest::StatusCode::BAD_GATEWAY,
				message: "failed to obtain csrfToken from server".to_string(),
				body: Some(body),
			}
		})?
		.to_string();

	let cookie_header = cookie_header_from_pairs(&cookies);
	return Ok((csrf, cookie_header));
	}

	Err(CliError::HttpStatus {
		status: reqwest::StatusCode::BAD_GATEWAY,
		message: "csrf request redirected too many times".to_string(),
		body: None,
	})
}

async fn nextauth_credentials_login(
	client: &reqwest::Client,
	base: &str,
	csrf_token: &str,
	csrf_cookie_header: &str,
	email: &str,
	password: &str,
	user_agent: &str,
	totp_code: Option<&str>,
) -> Result<LoginResponse, CliError> {
	let auth_base = auth_root_base(base);
	let callback_url = format!("{auth_base}/network");

	let mut form: Vec<(&str, String)> = vec![
		("csrfToken", csrf_token.to_string()),
		("callbackUrl", callback_url),
		("json", "true".to_string()),
		("email", email.to_string()),
		("password", password.to_string()),
		("userAgent", user_agent.to_string()),
	];

	if let Some(totp) = totp_code {
		form.push(("totpCode", totp.to_string()));
	}

	// NextAuth uses trailing slashes for many auth routes. Prefer the canonical
	// path to avoid 308 redirects (which preserve method and are easy to mishandle).
	let mut url = Url::parse(&format!("{auth_base}/api/auth/callback/credentials/"))?;
	let mut method = Method::POST;

	let mut cookies: BTreeMap<String, String> = parse_cookie_header_pairs(csrf_cookie_header);

	for _ in 0..8 {
		let cookie_header = cookie_header_from_pairs(&cookies);
		let mut request = client
			.request(method.clone(), url.clone())
			.header("accept", "application/json")
			.header("x-auth-return-redirect", "1")
			.header("user-agent", user_agent);

		if !cookie_header.is_empty() {
			request = request.header("cookie", cookie_header);
		}

		if method == Method::POST {
			request = request.form(&form);
		}

		let resp = request.send().await?;
		let status = resp.status();
		let set_cookies = collect_set_cookie(&resp);
		merge_set_cookie_pairs(&mut cookies, &set_cookies);

		let location = resp
			.headers()
			.get(reqwest::header::LOCATION)
			.and_then(|v| v.to_str().ok())
			.unwrap_or("")
			.trim()
			.to_string();

		let session_cookie = pick_cookie_value(
			&cookies,
			&[
				"__Secure-next-auth.session-token",
				"__Host-next-auth.session-token",
				"next-auth.session-token",
			],
		);

		let device_cookie = pick_cookie_value(
			&cookies,
			&[
				"__Secure-next-auth.did-token",
				"__Host-next-auth.did-token",
				"next-auth.did-token",
			],
		);

		if status.is_redirection() {
			if location.is_empty() {
				return Err(CliError::HttpStatus {
					status,
					message: "login redirect missing location header".to_string(),
					body: None,
				});
			}

			let error = parse_error_from_location(&location);
			if error.is_some() {
				return Ok(LoginResponse {
					ok: false,
					error,
					session_cookie,
					device_cookie,
				});
			}

			// If we already have a session cookie, we can stop here. Any further
			// redirect targets (like `/network`) are irrelevant for CLI auth.
			if session_cookie.is_some() {
				return Ok(LoginResponse {
					ok: true,
					error: None,
					session_cookie,
					device_cookie,
				});
			}

			url = resolve_redirect_url(&url, &location)?;

			// Match browser behavior:
			// - 307/308 preserve method and body (we keep POST for canonicalization hops)
			// - 301/302/303 switch to GET (NextAuth commonly redirects to callbackUrl)
			if status != reqwest::StatusCode::TEMPORARY_REDIRECT
				&& status != reqwest::StatusCode::PERMANENT_REDIRECT
			{
				method = Method::GET;
			}
			continue;
		}

		let body_text = resp.text().await.unwrap_or_default();

		if !status.is_success() {
			// If the server already issued a session cookie, treat login as successful
			// even if the next page fails (e.g. `/network` errors).
			if session_cookie.is_some() {
				return Ok(LoginResponse {
					ok: true,
					error: None,
					session_cookie,
					device_cookie,
				});
			}

			return Err(CliError::HttpStatus {
				status,
				message: "login request failed".to_string(),
				body: (!body_text.trim().is_empty()).then_some(body_text),
			});
		}

		let body_json = serde_json::from_str::<serde_json::Value>(&body_text).ok();

		let mut error = body_json
			.as_ref()
			.and_then(|v| v.get("error"))
			.and_then(|v| v.as_str())
			.map(str::to_string)
			.filter(|s| !s.trim().is_empty());

		if error.is_none() {
			let url_from_body = body_json
				.as_ref()
				.and_then(|v| v.get("url"))
				.and_then(|v| v.as_str())
				.unwrap_or("")
				.trim();
			error = parse_error_from_location(url_from_body);
		}

		let ok = error.is_none() && session_cookie.is_some();

		return Ok(LoginResponse {
			ok,
			error,
			session_cookie,
			device_cookie,
		});
	}

	Err(CliError::HttpStatus {
		status: reqwest::StatusCode::BAD_GATEWAY,
		message: "login redirected too many times".to_string(),
		body: None,
	})
}

fn auth_root_base(base: &str) -> String {
	let trimmed = base.trim_end_matches('/');
	trimmed
		.strip_suffix("/api")
		.filter(|value| !value.is_empty())
		.map_or_else(|| trimmed.to_string(), |value| value.to_string())
}

fn collect_set_cookie(resp: &reqwest::Response) -> Vec<String> {
	resp
		.headers()
		.get_all(reqwest::header::SET_COOKIE)
		.iter()
		.filter_map(|v| v.to_str().ok().map(str::to_string))
		.collect()
}

fn merge_set_cookie_pairs(out: &mut BTreeMap<String, String>, set_cookies: &[String]) {
	for raw in set_cookies {
		let Some((name, value)) = parse_set_cookie_pair(raw) else {
			continue;
		};
		out.insert(name, value);
	}
}

fn parse_set_cookie_pair(raw: &str) -> Option<(String, String)> {
	let pair = raw.split(';').next()?.trim();
	if pair.is_empty() {
		return None;
	}
	let (name, value) = pair.split_once('=')?;
	let name = name.trim();
	let value = value.trim();
	if name.is_empty() || value.is_empty() {
		return None;
	}
	Some((name.to_string(), value.to_string()))
}

fn cookie_header_from_pairs(pairs: &BTreeMap<String, String>) -> String {
	pairs
		.iter()
		.map(|(k, v)| format!("{k}={v}"))
		.collect::<Vec<_>>()
		.join("; ")
}

fn parse_cookie_header_pairs(header: &str) -> BTreeMap<String, String> {
	let mut out = BTreeMap::new();
	for part in header.split(';') {
		let part = part.trim();
		if part.is_empty() {
			continue;
		}
		let Some((name, value)) = part.split_once('=') else {
			continue;
		};
		let name = name.trim();
		let value = value.trim();
		if name.is_empty() || value.is_empty() {
			continue;
		}
		out.insert(name.to_string(), value.to_string());
	}
	out
}

fn pick_cookie_value(cookies: &BTreeMap<String, String>, names: &[&str]) -> Option<String> {
	for name in names {
		if let Some(value) = cookies.get(*name) {
			let v = value.trim();
			if !v.is_empty() {
				return Some(v.to_string());
			}
		}
	}
	None
}

fn resolve_redirect_url(current: &Url, location: &str) -> Result<Url, CliError> {
	if let Ok(abs) = Url::parse(location) {
		return Ok(abs);
	}
	Ok(current.join(location)?)
}

fn auth_hosts_list(
	cfg: &crate::config::Config,
	format: OutputFormat,
	global: &GlobalOpts,
) -> Result<(), CliError> {
	let mut hosts: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

	for host in cfg.host_defaults.keys() {
		hosts.entry(host.clone()).or_default();
	}

	for (name, profile) in &cfg.profiles {
		let Some(host_key) = canonical_host_key_opt(profile.host.as_deref()) else {
			continue;
		};
		hosts
			.entry(host_key)
			.or_default()
			.insert(name.clone());
	}

	let mut rows = Vec::with_capacity(hosts.len());
	for (host, profiles) in hosts {
		let default_profile = cfg.host_defaults.get(&host).cloned();
		let profiles: Vec<String> = profiles.into_iter().collect();
		rows.push(json!({
			"host": host,
			"default_profile": default_profile,
			"profiles": profiles,
		}));
	}

	output::print_value(&serde_json::Value::Array(rows), format, global.no_color)?;
	Ok(())
}

fn auth_hosts_set_default(
	global: &GlobalOpts,
	config_path: &std::path::Path,
	cfg: &mut crate::config::Config,
	effective: &crate::context::EffectiveConfig,
	args: crate::cli::AuthHostsSetDefaultArgs,
) -> Result<(), CliError> {
	let host_value = normalize_host_input(&args.host)?;
	let host_key = canonical_host_key(&host_value)?;

	let mut matching_profiles = Vec::new();
	for (name, profile) in &cfg.profiles {
		let Some(profile_key) = canonical_host_key_opt(profile.host.as_deref()) else {
			continue;
		};
		if profile_key == host_key {
			matching_profiles.push(name.clone());
		}
	}

	let profile = if let Some(profile) = args.profile {
		profile
	} else if matching_profiles.is_empty() {
		infer_profile_name(&host_value, cfg)?
	} else {
		matching_profiles
			.into_iter()
			.next()
			.expect("non-empty")
	};

	{
		let profile_cfg = cfg.profile_mut(&profile);
		match non_empty(profile_cfg.host.clone()) {
			Some(existing) => {
				let existing_key = canonical_host_key(&existing)?;
				if existing_key != host_key {
					return Err(CliError::InvalidArgument(format!(
						"profile '{profile}' is configured for '{existing}', but the target host is '{host_value}'",
					)));
				}
			}
			None => {
				profile_cfg.host = Some(host_value.clone());
			}
		}
	}

	cfg.host_defaults.insert(host_key.clone(), profile.clone());
	config::save_config(config_path, cfg)?;

	if !global.quiet {
		eprintln!("Default profile for '{host_key}' set to '{profile}'.");
	}

	let value = json!({
		"host": host_key,
		"default_profile": profile,
	});
	output::print_value(&value, effective.output, global.no_color)?;
	Ok(())
}

fn auth_hosts_unset_default(
	global: &GlobalOpts,
	config_path: &std::path::Path,
	cfg: &mut crate::config::Config,
	effective: &crate::context::EffectiveConfig,
	args: crate::cli::AuthHostsUnsetDefaultArgs,
) -> Result<(), CliError> {
	let host_value = normalize_host_input(&args.host)?;
	let host_key = canonical_host_key(&host_value)?;

	let removed = cfg.host_defaults.remove(&host_key).is_some();
	config::save_config(config_path, cfg)?;

	if !global.quiet {
		if removed {
			eprintln!("Default profile for '{host_key}' removed.");
		} else {
			eprintln!("No default profile configured for '{host_key}'.");
		}
	}

	let value = json!({
		"host": host_key,
		"removed": removed,
	});
	output::print_value(&value, effective.output, global.no_color)?;
	Ok(())
}

fn infer_profile_name(host: &str, cfg: &crate::config::Config) -> Result<String, CliError> {
	let url = Url::parse(host.trim())
		.map_err(|err| CliError::InvalidArgument(format!("invalid host url: {err}")))?;

	let Some(hostname) = url.host_str() else {
		return Err(CliError::InvalidArgument(format!(
			"invalid host url: missing hostname in '{host}'"
		)));
	};

	let scheme = url.scheme().to_ascii_lowercase();
	let default_port = match scheme.as_str() {
		"http" => Some(80),
		"https" => Some(443),
		_ => None,
	};

	let port = url.port();
	let include_port = match (port, default_port) {
		(Some(p), Some(d)) => p != d,
		(Some(_), None) => true,
		(None, _) => false,
	};

	let mut base = slugify_profile_name(hostname);
	if base.is_empty() {
		base = "host".to_string();
	}

	if include_port {
		base.push('-');
		base.push_str(&port.expect("include_port implies Some").to_string());
	}

	if !cfg.profiles.contains_key(&base) {
		return Ok(base);
	}

	for n in 2.. {
		let candidate = format!("{base}-{n}");
		if !cfg.profiles.contains_key(&candidate) {
			return Ok(candidate);
		}
	}

	unreachable!("infinite loop must return")
}

fn slugify_profile_name(value: &str) -> String {
	let mut out = String::new();
	let mut prev_dash = false;
	for ch in value.chars() {
		let lower = ch.to_ascii_lowercase();
		if lower.is_ascii_lowercase() || lower.is_ascii_digit() {
			out.push(lower);
			prev_dash = false;
			continue;
		}

		if !prev_dash && !out.is_empty() {
			out.push('-');
			prev_dash = true;
		}
	}
	out.trim_matches('-').to_string()
}

fn non_empty(value: Option<String>) -> Option<String> {
	match value {
		Some(v) if v.trim().is_empty() => None,
		other => other,
	}
}

fn explicit_host_override(global: &GlobalOpts) -> Option<String> {
	global
		.host
		.clone()
		.or_else(|| env::var("ZTNET_HOST").ok())
		.or_else(|| env::var("API_ADDRESS").ok())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::config::ProfileConfig;

	#[test]
	fn infer_profile_name_slugifies_host() {
		let cfg = crate::config::Config::default();
		assert_eq!(
			infer_profile_name("https://ztnet.example.com", &cfg).unwrap(),
			"ztnet-example-com"
		);
		assert_eq!(
			infer_profile_name("http://localhost:3000", &cfg).unwrap(),
			"localhost-3000"
		);
	}

	#[test]
	fn infer_profile_name_omits_default_port() {
		let cfg = crate::config::Config::default();
		assert_eq!(
			infer_profile_name("https://example.com:443", &cfg).unwrap(),
			"example-com"
		);
	}

	#[test]
	fn infer_profile_name_ensures_uniqueness() {
		let mut cfg = crate::config::Config::default();
		cfg.profiles.insert(
			"localhost-3000".to_string(),
			ProfileConfig {
				host: Some("http://localhost:3000".to_string()),
				..Default::default()
			},
		);

		assert_eq!(
			infer_profile_name("http://localhost:3000", &cfg).unwrap(),
			"localhost-3000-2"
		);
	}

	#[test]
	fn parse_set_cookie_pair_extracts_cookie_name_and_value() {
		let (k, v) = parse_set_cookie_pair("next-auth.csrf-token=abc%7Cdef; Path=/; HttpOnly")
			.expect("pair");
		assert_eq!(k, "next-auth.csrf-token");
		assert_eq!(v, "abc%7Cdef");
	}

	#[test]
	fn merge_set_cookie_pairs_overwrites_previous_values() {
		let mut out = BTreeMap::new();
		merge_set_cookie_pairs(
			&mut out,
			&[
				"next-auth.csrf-token=one; Path=/".to_string(),
				"next-auth.csrf-token=two; Path=/".to_string(),
			],
		);
		assert_eq!(out.get("next-auth.csrf-token").map(String::as_str), Some("two"));
	}

	#[test]
	fn resolve_redirect_url_handles_relative_and_absolute_locations() {
		let current = Url::parse("https://example.com/api/auth/csrf").unwrap();
		let joined = resolve_redirect_url(&current, "/api/auth/csrf/").unwrap();
		assert_eq!(joined.as_str(), "https://example.com/api/auth/csrf/");

		let absolute =
			resolve_redirect_url(&current, "https://other.example.com/api/auth/csrf").unwrap();
		assert_eq!(absolute.as_str(), "https://other.example.com/api/auth/csrf");
	}
}

fn parse_error_from_location(location: &str) -> Option<String> {
	let (_, query) = location.split_once('?')?;
	for part in query.split('&') {
		let (k, v) = part.split_once('=')?;
		if k == "error" {
			return Some(v.to_string());
		}
	}
	None
}
