//! Operator entry point for BeamRS load testing.
//!
//! Builds the real application router against PostgreSQL and drives it with a
//! named scenario. Measurement is in-process: it covers handler and database
//! latency, not kernel networking or HTTP wire parsing.

use std::{process::ExitCode, sync::Arc, time::Duration};

use anyhow::Result;
use beamrs::{
    config::Settings,
    loadtest::{run_load_test, LoadProfile, RequestSpec},
    repository::BeamRepository,
    server::{router, AppState},
};
use leptos::config::get_configuration;

const USAGE: &str = "\
Usage: loadtest [options]

Options:
  --scenario NAME     health | read-mix | write-mix   (default: read-mix)
  --concurrency N     concurrent workers              (default: 16)
  --requests N        total requests to send          (default: 200)
  --frequency-id N    frequency used by read requests (default: 1)
  --username NAME     user used by profile requests   (default: Anon1)
  --json              print the report as JSON
  -h, --help          print this message

DATABASE_URL must point at a BeamRS database. The process exits with status 1
when any request fails.
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct Options {
    scenario: Scenario,
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

    let report = run_load_test(router(state), &profile).await?;
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
            "--concurrency",
            "4",
            "--requests",
            "40",
            "--frequency-id",
            "7",
            "--username",
            "Anon42",
            "--json",
        ]);

        assert_eq!(parsed.scenario, Scenario::WriteMix);
        assert_eq!(parsed.concurrency, 4);
        assert_eq!(parsed.requests, 40);
        assert_eq!(parsed.frequency_id, 7);
        assert_eq!(parsed.username, "Anon42");
        assert!(parsed.json);
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
