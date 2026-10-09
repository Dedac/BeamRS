//! Operator entry point for BeamRS load testing.
//!
//! Drives BeamRS with a named scenario over one of three paths:
//!
//! - `--transport in-process` (default) builds the real router against
//!   PostgreSQL and calls it without sockets: handler and database latency only.
//! - `--transport http` builds the same router, serves it on an ephemeral
//!   loopback port inside this process, and sends HTTP/1.1 requests over TCP.
//! - `--target URL` sends HTTP/1.1 requests to an already running server and
//!   does not connect to a database itself.

use std::{process::ExitCode, sync::Arc, time::Duration};

use anyhow::Result;
use beamrs::{
    config::Settings,
    loadtest::{
        run_load_test_with, serve_loopback, HttpTarget, LoadProfile, RequestSpec, Transport,
        DEFAULT_HTTP_TIMEOUT,
    },
    repository::BeamRepository,
    server::{router, AppState},
};
use leptos::config::get_configuration;

const USAGE: &str = "\
Usage: loadtest [options]

Options:
  --scenario NAME     health | read-mix | write-mix   (default: read-mix)
  --transport NAME    in-process | http               (default: in-process)
  --target URL        load a running server at URL over HTTP, for example
                      http://127.0.0.1:8080 (implies --transport http)
  --http-timeout SEC  deadline for a complete HTTP request and response body
                      (default: 30; HTTP transport only)
  --concurrency N     concurrent workers              (default: 16)
  --requests N        total requests to send          (default: 200)
  --frequency-id N    frequency used by read requests (default: 1)
  --username NAME     user used by profile requests   (default: Anon1)
  --json              print the report as JSON
  -h, --help          print this message

DATABASE_URL must point at a BeamRS database unless --target is given. The
process exits with status 1 when any request fails.
";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    Health,
    ReadMix,
    WriteMix,
}

impl Scenario {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "health" => Ok(Self::Health),
            "read-mix" => Ok(Self::ReadMix),
            "write-mix" => Ok(Self::WriteMix),
            other => Err(format!(
                "unknown scenario '{other}' (expected health, read-mix, or write-mix)"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TransportKind {
    InProcess,
    Http,
}

impl TransportKind {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "in-process" => Ok(Self::InProcess),
            "http" => Ok(Self::Http),
            other => Err(format!(
                "unknown transport '{other}' (expected in-process or http)"
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Options {
    scenario: Scenario,
    transport: TransportKind,
    target: Option<String>,
    http_timeout_seconds: u64,
    concurrency: usize,
    requests: usize,
    frequency_id: i32,
    username: String,
    json: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            scenario: Scenario::ReadMix,
            transport: TransportKind::InProcess,
            target: None,
            http_timeout_seconds: DEFAULT_HTTP_TIMEOUT.as_secs(),
            concurrency: 16,
            requests: 200,
            frequency_id: 1,
            username: "Anon1".to_string(),
            json: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Command {
    Help,
    Run(Box<Options>),
}

fn parse_args(args: &[String]) -> Result<Command, String> {
    let value_at = |index: usize| -> Result<String, String> {
        args.get(index + 1)
            .cloned()
            .ok_or_else(|| format!("{} requires a value", args[index]))
    };

    let mut options = Options::default();
    let mut explicit_transport = None;
    let mut http_timeout_set = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--json" => {
                options.json = true;
                index += 1;
            }
            "--scenario" => {
                options.scenario = Scenario::parse(&value_at(index)?)?;
                index += 2;
            }
            "--transport" => {
                explicit_transport = Some(TransportKind::parse(&value_at(index)?)?);
                index += 2;
            }
            "--target" => {
                options.target = Some(value_at(index)?);
                index += 2;
            }
            "--http-timeout" => {
                let seconds = parse_number::<u64>("--http-timeout", &value_at(index)?)?;
                if seconds == 0 {
                    return Err("--http-timeout must be at least 1 second".to_string());
                }
                options.http_timeout_seconds = seconds;
                http_timeout_set = true;
                index += 2;
            }
            "--concurrency" => {
                options.concurrency = parse_number("--concurrency", &value_at(index)?)?;
                index += 2;
            }
            "--requests" => {
                options.requests = parse_number("--requests", &value_at(index)?)?;
                index += 2;
            }
            "--frequency-id" => {
                options.frequency_id = parse_number("--frequency-id", &value_at(index)?)?;
                index += 2;
            }
            "--username" => {
                options.username = value_at(index)?;
                index += 2;
            }
            other => return Err(format!("unknown option '{other}'")),
        }
    }

    options.transport = match (explicit_transport, &options.target) {
        (Some(TransportKind::InProcess), Some(_)) => {
            return Err("--target cannot be combined with --transport in-process".to_string())
        }
        (Some(kind), None) => kind,
        (_, Some(_)) => TransportKind::Http,
        (None, None) => TransportKind::InProcess,
    };

    if http_timeout_set && options.transport == TransportKind::InProcess {
        return Err("--http-timeout requires --transport http or --target".to_string());
    }

    Ok(Command::Run(Box::new(options)))
}

fn parse_number<T: std::str::FromStr>(flag: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} expects a number, got '{value}'"))
}

fn build_profile(options: &Options) -> LoadProfile {
    let frequency_rays = format!("/api/frequencies/{}/rays", options.frequency_id);
    let encoded = urlencoding::encode(&options.username).into_owned();
    let user_rays = format!("/api/users/{encoded}/rays");
    let user_prisms = format!("/api/users/{encoded}/prisms");
    let user_body = format!(
        "{{\"username\":{}}}",
        serde_json::to_string(&options.username).unwrap_or_else(|_| "\"Anon1\"".to_string())
    );

    let requests = match options.scenario {
        Scenario::Health => vec![RequestSpec::get("health", "/health")],
        Scenario::ReadMix => vec![
            RequestSpec::get("list-frequencies", "/api/frequencies").with_weight(3),
            RequestSpec::get("frequency-rays", &frequency_rays).with_weight(3),
            RequestSpec::get("user-rays", &user_rays).with_weight(2),
            RequestSpec::get("user-prisms", &user_prisms).with_weight(1),
            RequestSpec::get("health", "/health").with_weight(1),
        ],
        // The user route is a get-or-create upsert, so repeating it exercises
        // the write path without growing the database on every request.
        Scenario::WriteMix => vec![
            RequestSpec::get("list-frequencies", "/api/frequencies").with_weight(2),
            RequestSpec::get("frequency-rays", &frequency_rays).with_weight(2),
            RequestSpec::get("user-rays", &user_rays).with_weight(1),
            RequestSpec::post_json("upsert-user", "/api/users", &user_body).with_weight(3),
        ],
    };

    LoadProfile::new(options.concurrency, options.requests, requests)
}

async fn run(options: &Options) -> Result<ExitCode> {
    let profile = build_profile(options);
    profile.validate()?;

    if let Some(url) = &options.target {
        let target =
            HttpTarget::parse(url)?.with_timeout(Duration::from_secs(options.http_timeout_seconds));
        let transport = Transport::Http(target);
        return report(options, &transport, &profile).await;
    }

    // This binary is not launched by cargo-leptos, which normally exports the
    // generated-asset configuration. Supply the package default so the router
    // builds with the same options the server uses.
    if std::env::var_os("LEPTOS_OUTPUT_NAME").is_none() {
        std::env::set_var("LEPTOS_OUTPUT_NAME", "beamrs");
    }

    let settings = Settings::from_env()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(options.concurrency.clamp(1, 64) as u32)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&settings.database_url)
        .await?;
    BeamRepository::initialize(&pool).await?;

    let state = AppState {
        repo: Arc::new(BeamRepository::new(pool)),
        leptos_options: get_configuration(None)?.leptos_options,
    };

    let app = router(state);
    match options.transport {
        TransportKind::InProcess => report(options, &Transport::InProcess(app), &profile).await,
        TransportKind::Http => {
            let (target, server) = serve_loopback(app).await?;
            let target = target.with_timeout(Duration::from_secs(options.http_timeout_seconds));
            let result = report(options, &Transport::Http(target), &profile).await;
            server.abort();
            result
        }
    }
}

async fn report(
    options: &Options,
    transport: &Transport,
    profile: &LoadProfile,
) -> Result<ExitCode> {
    let report = run_load_test_with(transport, profile).await?;
    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", report.render_text());
    }

    Ok(if report.failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match parse_args(&args) {
        Ok(command) => command,
        Err(message) => {
            eprintln!("loadtest: {message}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    let options = match command {
        Command::Help => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Command::Run(options) => options,
    };

    match run(&options).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("loadtest failed: {error:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn options(values: &[&str]) -> Options {
        match parse_args(&args(values)).unwrap() {
            Command::Run(options) => *options,
            Command::Help => panic!("expected a run command"),
        }
    }

    #[test]
    fn defaults_are_used_without_arguments() {
        assert_eq!(options(&[]), Options::default());
    }

    #[test]
    fn every_flag_is_parsed() {
        let parsed = options(&[
            "--scenario",
            "write-mix",
            "--transport",
            "http",
            "--concurrency",
            "4",
            "--requests",
            "40",
            "--frequency-id",
            "7",
            "--username",
            "Anon42",
            "--http-timeout",
            "45",
            "--json",
        ]);

        assert_eq!(parsed.scenario, Scenario::WriteMix);
        assert_eq!(parsed.concurrency, 4);
        assert_eq!(parsed.requests, 40);
        assert_eq!(parsed.frequency_id, 7);
        assert_eq!(parsed.username, "Anon42");
        assert_eq!(parsed.http_timeout_seconds, 45);
        assert!(parsed.json);
    }

    #[test]
    fn http_timeout_must_be_positive() {
        let error = parse_args(&args(&["--http-timeout", "0"])).unwrap_err();
        assert!(error.contains("at least 1 second"));
    }

    #[test]
    fn http_timeout_requires_an_http_transport() {
        let error = parse_args(&args(&["--http-timeout", "5"])).unwrap_err();
        assert!(error.contains("requires --transport http or --target"));
        assert_eq!(
            options(&["--transport", "http", "--http-timeout", "5"]).http_timeout_seconds,
            5
        );
    }

    #[test]
    fn help_is_recognized() {
        assert_eq!(parse_args(&args(&["--help"])).unwrap(), Command::Help);
        assert_eq!(parse_args(&args(&["-h"])).unwrap(), Command::Help);
    }

    #[test]
    fn unknown_option_is_rejected() {
        let error = parse_args(&args(&["--nope"])).unwrap_err();
        assert!(error.contains("unknown option"));
    }

    #[test]
    fn missing_value_is_rejected() {
        let error = parse_args(&args(&["--requests"])).unwrap_err();
        assert!(error.contains("requires a value"));
    }

    #[test]
    fn non_numeric_value_is_rejected() {
        let error = parse_args(&args(&["--concurrency", "many"])).unwrap_err();
        assert!(error.contains("expects a number"));
    }

    #[test]
    fn transport_defaults_to_in_process_and_accepts_http() {
        assert_eq!(options(&[]).transport, TransportKind::InProcess);
        assert_eq!(
            options(&["--transport", "http"]).transport,
            TransportKind::Http
        );
        let error = parse_args(&args(&["--transport", "carrier-pigeon"])).unwrap_err();
        assert!(error.contains("unknown transport"));
    }

    #[test]
    fn target_implies_http_transport() {
        let parsed = options(&["--target", "http://127.0.0.1:8080"]);
        assert_eq!(parsed.transport, TransportKind::Http);
        assert_eq!(parsed.target.as_deref(), Some("http://127.0.0.1:8080"));
        assert_eq!(
            options(&["--transport", "http", "--target", "http://localhost:1"]).transport,
            TransportKind::Http
        );
    }

    #[test]
    fn target_conflicts_with_in_process_transport() {
        let error = parse_args(&args(&[
            "--transport",
            "in-process",
            "--target",
            "http://127.0.0.1:8080",
        ]))
        .unwrap_err();
        assert!(error.contains("cannot be combined"));
    }

    #[test]
    fn unknown_scenario_is_rejected() {
        let error = parse_args(&args(&["--scenario", "stress"])).unwrap_err();
        assert!(error.contains("unknown scenario"));
    }

    #[test]
    fn every_scenario_builds_a_valid_profile() {
        for scenario in ["health", "read-mix", "write-mix"] {
            let parsed = options(&["--scenario", scenario]);
            let profile = build_profile(&parsed);

            assert!(profile.validate().is_ok(), "{scenario} profile is invalid");
            assert!(!profile.requests.is_empty());
        }
    }

    #[test]
    fn usernames_are_percent_encoded_in_paths() {
        let parsed = options(&["--username", "Anon 42"]);
        let profile = build_profile(&parsed);

        assert!(profile
            .requests
            .iter()
            .any(|spec| spec.path == "/api/users/Anon%2042/rays"));
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn write_mix_only_uses_idempotent_writes() {
        let profile = build_profile(&options(&["--scenario", "write-mix"]));
        let writes: Vec<&RequestSpec> = profile
            .requests
            .iter()
            .filter(|spec| spec.body.is_some())
            .collect();

        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].path, "/api/users");
    }
}
