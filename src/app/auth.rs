use reqwest::Method;
use serde_json::json;

use crate::cli::{AuthCommand, GlobalOpts, OutputFormat};
use crate::config;
use crate::context::resolve_effective_config;
use crate::error::CliError;
use crate::http::HttpClient;
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

			cfg.profile_mut(&profile).token = Some(token);
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

			if args.password_stdin && args.password.is_some() {
				return Err(CliError::InvalidArgument(
					"cannot combine --password-stdin with --password".to_string(),
				));
			}

			let password = if args.password_stdin {
				read_stdin_trimmed()?
			} else {
				args.password.ok_or_else(|| {
					CliError::InvalidArgument("missing --password (or --password-stdin)".to_string())
				})?
			};

			if password.trim().is_empty() {
				return Err(CliError::InvalidArgument("password cannot be empty".to_string()));
			}

			if global.dry_run {
				let base = effective.host.trim_end_matches('/');
				println!("POST {base}/api/auth/callback/credentials");
				println!("content-type: application/x-www-form-urlencoded");
				println!("(credentials omitted)");
				return Err(CliError::DryRunPrinted);
			}

			let base = effective.host.trim_end_matches('/');

			let client = reqwest::Client::builder()
				.timeout(effective.timeout)
				.redirect(reqwest::redirect::Policy::none())
				.build()?;

			let mut totp = args.totp.clone();
			loop {
				let (csrf_token, csrf_cookie_header) =
					fetch_nextauth_csrf(&client, base).await?;

				let user_agent = format!("ztnet-cli/{}", env!("CARGO_PKG_VERSION"));
				let response = nextauth_credentials_login(
					&client,
					base,
					&csrf_token,
					&csrf_cookie_header,
					&args.email,
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
					profile_cfg.session_cookie = Some(session);
					profile_cfg.device_cookie = response.device_cookie;
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
) -> Result<(String, String), CliError> {
	let url = format!("{base}/api/auth/csrf");
	let resp = client.get(url).header("accept", "application/json").send().await?;
	let set_cookies = collect_set_cookie(&resp);

	let value = resp.json::<serde_json::Value>().await?;
	let csrf = value
		.get("csrfToken")
		.and_then(|v| v.as_str())
		.ok_or_else(|| {
			CliError::HttpStatus {
				status: reqwest::StatusCode::BAD_GATEWAY,
				message: "failed to obtain csrfToken from server".to_string(),
				body: Some(value.to_string()),
			}
		})?
		.to_string();

	let cookie_header = set_cookie_to_cookie_header(&set_cookies);
	Ok((csrf, cookie_header))
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
	let url = format!("{base}/api/auth/callback/credentials");
	let callback_url = format!("{base}/network");

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

	let resp = client
		.post(url)
		.header("accept", "application/json")
		.header("cookie", csrf_cookie_header)
		.form(&form)
		.send()
		.await?;

	let status = resp.status();
	let set_cookies = collect_set_cookie(&resp);
	let location = resp
		.headers()
		.get(reqwest::header::LOCATION)
		.and_then(|v| v.to_str().ok())
		.unwrap_or("")
		.to_string();

	let session_cookie = extract_cookie_value(&set_cookies, "next-auth.session-token")
		.or_else(|| extract_cookie_value(&set_cookies, "__Secure-next-auth.session-token"));

	let device_cookie = extract_cookie_value(&set_cookies, "next-auth.did-token");

	if status.is_success() {
		let value = resp
			.json::<serde_json::Value>()
			.await
			.unwrap_or(serde_json::Value::Null);

		let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
		let error = value
			.get("error")
			.and_then(|v| v.as_str())
			.map(str::to_string)
			.filter(|s| !s.trim().is_empty());

		return Ok(LoginResponse {
			ok,
			error,
			session_cookie,
			device_cookie,
		});
	}

	if status.is_redirection() {
		let error = parse_error_from_location(&location);
		let ok = error.is_none() && session_cookie.is_some();

		return Ok(LoginResponse {
			ok,
			error,
			session_cookie,
			device_cookie,
		});
	}

	Err(CliError::HttpStatus {
		status,
		message: "login request failed".to_string(),
		body: resp.text().await.ok(),
	})
}

fn collect_set_cookie(resp: &reqwest::Response) -> Vec<String> {
	resp
		.headers()
		.get_all(reqwest::header::SET_COOKIE)
		.iter()
		.filter_map(|v| v.to_str().ok().map(str::to_string))
		.collect()
}

fn set_cookie_to_cookie_header(set_cookies: &[String]) -> String {
	let mut pairs = Vec::new();
	for raw in set_cookies {
		let Some((pair, _rest)) = raw.split_once(';') else {
			continue;
		};
		let pair = pair.trim();
		if pair.is_empty() {
			continue;
		}
		pairs.push(pair.to_string());
	}
	pairs.join("; ")
}

fn extract_cookie_value(set_cookies: &[String], name: &str) -> Option<String> {
	let prefix = format!("{name}=");
	for raw in set_cookies {
		let trimmed = raw.trim();
		if !trimmed.starts_with(&prefix) {
			continue;
		}
		let rest = &trimmed[prefix.len()..];
		let value = rest.split(';').next().unwrap_or("").trim();
		if !value.is_empty() {
			return Some(value.to_string());
		}
	}
	None
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
