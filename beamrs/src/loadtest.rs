//! In-process load testing for the BeamRS application router.
//!
//! The engine drives an [`axum::Router`] directly through
//! [`tower::ServiceExt::oneshot`] instead of opening sockets. That exercises
//! the real extractors, handlers, error mapping, and [`BeamStore`] calls while
//! excluding kernel networking, TLS, and HTTP wire parsing, so the reported
//! latency is application latency rather than end-to-end client latency.
//!
//! [`BeamStore`]: crate::repository::BeamStore

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use axum::{
    body::{to_bytes, Body},
    http::{header::CONTENT_TYPE, Method, Request},
    Router,
};
use serde::Serialize;
use thiserror::Error;
use tower::ServiceExt;

/// Responses at or above this status are counted as failures.
const FAILURE_STATUS: u16 = 400;

#[derive(Debug, Error)]
pub enum LoadTestError {
    #[error("concurrency must be at least 1")]
    ZeroConcurrency,
    #[error("total_requests must be at least 1")]
    ZeroRequests,
    #[error("the request mix must contain at least one request")]
    EmptyMix,
    #[error("the request mix must contain at least one request with a non-zero weight")]
    ZeroWeightMix,
    #[error("request '{name}' is not valid: {reason}")]
    InvalidRequest { name: String, reason: String },
    #[error("a load worker failed: {0}")]
    Worker(String),
}

/// One named request in a load profile.
#[derive(Clone, Debug)]
pub struct RequestSpec {
    pub name: String,
    pub method: Method,
    pub path: String,
    pub body: Option<String>,
    /// Relative share of the generated requests. A spec with weight 0 is never sent.
    pub weight: u32,
}

impl RequestSpec {
    pub fn get(name: &str, path: &str) -> Self {
        Self {
            name: name.to_string(),
            method: Method::GET,
            path: path.to_string(),
            body: None,
            weight: 1,
        }
    }

    pub fn post_json(name: &str, path: &str, body: &str) -> Self {
        Self {
            name: name.to_string(),
            method: Method::POST,
            path: path.to_string(),
            body: Some(body.to_string()),
            weight: 1,
        }
    }

    pub fn with_weight(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }

    fn build(&self) -> Result<Request<Body>, LoadTestError> {
        let builder = Request::builder()
            .method(self.method.clone())
            .uri(&self.path);
        let request = match &self.body {
            Some(body) => builder
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body.clone())),
            None => builder.body(Body::empty()),
        };
        request.map_err(|error| LoadTestError::InvalidRequest {
            name: self.name.clone(),
            reason: error.to_string(),
        })
    }
}

/// How much load to generate and which requests to generate.
#[derive(Clone, Debug)]
pub struct LoadProfile {
    pub concurrency: usize,
    pub total_requests: usize,
    pub requests: Vec<RequestSpec>,
}

impl LoadProfile {
    pub fn new(concurrency: usize, total_requests: usize, requests: Vec<RequestSpec>) -> Self {
        Self {
            concurrency,
            total_requests,
            requests,
        }
    }

    /// Reject profiles that cannot produce a meaningful report.
    pub fn validate(&self) -> Result<(), LoadTestError> {
        if self.concurrency == 0 {
            return Err(LoadTestError::ZeroConcurrency);
        }
        if self.total_requests == 0 {
            return Err(LoadTestError::ZeroRequests);
        }
        if self.requests.is_empty() {
            return Err(LoadTestError::EmptyMix);
        }
        if self.requests.iter().all(|spec| spec.weight == 0) {
            return Err(LoadTestError::ZeroWeightMix);
        }
        for spec in &self.requests {
            spec.build()?;
        }
        Ok(())
    }

    /// Expand weights into a deterministic schedule of request indices.
    fn schedule(&self) -> Vec<usize> {
        let mut schedule = Vec::new();
        for (index, spec) in self.requests.iter().enumerate() {
            for _ in 0..spec.weight {
                schedule.push(index);
            }
        }
        schedule
    }
}

#[derive(Clone, Debug)]
struct Sample {
    name: String,
    status: Option<u16>,
    latency: Duration,
}

impl Sample {
    fn failed(&self) -> bool {
        match self.status {
            Some(status) => status >= FAILURE_STATUS,
            None => true,
        }
    }
}

/// Latency distribution for a sample set, in milliseconds.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct LatencyStats {
    pub count: usize,
    pub failures: usize,
    pub mean_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
}

impl LatencyStats {
    fn from_samples(samples: &[&Sample]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }
        let mut millis: Vec<f64> = samples
            .iter()
            .map(|sample| sample.latency.as_secs_f64() * 1000.0)
            .collect();
        millis.sort_by(|left, right| left.total_cmp(right));
        let total: f64 = millis.iter().sum();
        Self {
            count: samples.len(),
            failures: samples.iter().filter(|sample| sample.failed()).count(),
            mean_ms: total / millis.len() as f64,
            p50_ms: percentile(&millis, 50.0),
            p95_ms: percentile(&millis, 95.0),
            p99_ms: percentile(&millis, 99.0),
            max_ms: millis[millis.len() - 1],
        }
    }
}

/// Nearest-rank percentile over an ascending slice.
fn percentile(sorted_millis: &[f64], percent: f64) -> f64 {
    if sorted_millis.is_empty() {
        return 0.0;
    }
    let rank = (percent / 100.0 * sorted_millis.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted_millis.len() - 1);
    sorted_millis[index]
}

/// Aggregated result of one load run.
#[derive(Clone, Debug, Serialize)]
pub struct LoadReport {
    pub concurrency: usize,
    pub elapsed_ms: f64,
    pub total_requests: usize,
    pub successes: usize,
    pub failures: usize,
    pub requests_per_second: f64,
    pub status_counts: BTreeMap<String, usize>,
    pub overall: LatencyStats,
    pub endpoints: BTreeMap<String, LatencyStats>,
}

impl LoadReport {
    fn from_samples(concurrency: usize, elapsed: Duration, samples: Vec<Sample>) -> Self {
        let borrowed: Vec<&Sample> = samples.iter().collect();
        let overall = LatencyStats::from_samples(&borrowed);

        let mut status_counts: BTreeMap<String, usize> = BTreeMap::new();
        for sample in &samples {
            let key = match sample.status {
                Some(status) => status.to_string(),
                None => "request-build-error".to_string(),
            };
            *status_counts.entry(key).or_default() += 1;
        }

        let mut grouped: BTreeMap<String, Vec<&Sample>> = BTreeMap::new();
        for sample in &samples {
            grouped.entry(sample.name.clone()).or_default().push(sample);
        }
        let endpoints = grouped
            .into_iter()
            .map(|(name, group)| (name, LatencyStats::from_samples(&group)))
            .collect();

        let elapsed_secs = elapsed.as_secs_f64();
        let total_requests = samples.len();
        Self {
            concurrency,
            elapsed_ms: elapsed_secs * 1000.0,
            total_requests,
            successes: total_requests - overall.failures,
            failures: overall.failures,
            requests_per_second: if elapsed_secs > 0.0 {
                total_requests as f64 / elapsed_secs
            } else {
                0.0
            },
            status_counts,
            overall,
            endpoints,
        }
    }

    /// Human-readable report for operators.
    pub fn render_text(&self) -> String {
        let mut out = String::new();
        out.push_str("BeamRS load test\n");
        out.push_str(&format!(
            "  concurrency      {}\n  requests         {}\n  elapsed          {:.1} ms\n  throughput       {:.1} req/s\n  successes        {}\n  failures         {}\n",
            self.concurrency,
            self.total_requests,
            self.elapsed_ms,
            self.requests_per_second,
            self.successes,
            self.failures,
        ));

        out.push_str("  status counts    ");
        let statuses: Vec<String> = self
            .status_counts
            .iter()
            .map(|(status, count)| format!("{status}={count}"))
            .collect();
        out.push_str(&statuses.join(" "));
        out.push('\n');

        out.push_str(&format!(
            "\n{:<28}{:>8}{:>9}{:>10}{:>10}{:>10}{:>10}{:>10}\n",
            "endpoint", "count", "failed", "mean ms", "p50 ms", "p95 ms", "p99 ms", "max ms"
        ));
        let mut rows: Vec<(&str, &LatencyStats)> = vec![("(all)", &self.overall)];
        rows.extend(
            self.endpoints
                .iter()
                .map(|(name, stats)| (name.as_str(), stats)),
        );
        for (name, stats) in rows {
            out.push_str(&format!(
                "{:<28}{:>8}{:>9}{:>10.2}{:>10.2}{:>10.2}{:>10.2}{:>10.2}\n",
                name,
                stats.count,
                stats.failures,
                stats.mean_ms,
                stats.p50_ms,
                stats.p95_ms,
                stats.p99_ms,
                stats.max_ms,
            ));
        }
        out
    }
}

/// Drive `router` with `profile` and aggregate the results.
///
/// Each of `profile.concurrency` workers claims request slots from a shared
/// counter until `profile.total_requests` is exhausted, so exactly that many
/// requests are sent regardless of how unevenly they complete.
pub async fn run_load_test(
    router: Router,
    profile: &LoadProfile,
) -> Result<LoadReport, LoadTestError> {
    profile.validate()?;

    let schedule = Arc::new(profile.schedule());
    let specs = Arc::new(profile.requests.clone());
    let cursor = Arc::new(AtomicUsize::new(0));
    let total_requests = profile.total_requests;

    let started = Instant::now();
    let mut workers = Vec::with_capacity(profile.concurrency);
    for _ in 0..profile.concurrency {
        let router = router.clone();
        let schedule = Arc::clone(&schedule);
        let specs = Arc::clone(&specs);
        let cursor = Arc::clone(&cursor);
        workers.push(tokio::spawn(async move {
            let mut samples = Vec::new();
            loop {
                let slot = cursor.fetch_add(1, Ordering::Relaxed);
                if slot >= total_requests {
                    break;
                }
                let spec = &specs[schedule[slot % schedule.len()]];
                let request = match spec.build() {
                    Ok(request) => request,
                    Err(_) => {
                        samples.push(Sample {
                            name: spec.name.clone(),
                            status: None,
                            latency: Duration::ZERO,
                        });
                        continue;
                    }
                };

                let call_started = Instant::now();
                let status = match router.clone().oneshot(request).await {
                    Ok(response) => {
                        let status = response.status().as_u16();
                        // Drain the body so the measurement covers the full response.
                        let _ = to_bytes(response.into_body(), usize::MAX).await;
                        Some(status)
                    }
                    Err(_) => None,
                };
                samples.push(Sample {
                    name: spec.name.clone(),
                    status,
                    latency: call_started.elapsed(),
                });
            }
            samples
        }));
    }

    let mut samples = Vec::with_capacity(total_requests);
    for worker in workers {
        samples.extend(
            worker
                .await
                .map_err(|error| LoadTestError::Worker(error.to_string()))?,
        );
    }
    let elapsed = started.elapsed();

    Ok(LoadReport::from_samples(
        profile.concurrency,
        elapsed,
        samples,
    ))
}

#[cfg(test)]
mod tests {
    use axum::{
        http::StatusCode,
        routing::{get, post},
        Router,
    };

    use super::*;

    fn test_router() -> Router {
        Router::new()
            .route("/ok", get(|| async { StatusCode::OK }))
            .route("/missing", get(|| async { StatusCode::NOT_FOUND }))
            .route("/boom", get(|| async { StatusCode::INTERNAL_SERVER_ERROR }))
            .route("/echo", post(|body: String| async move { body }))
    }

    fn profile(requests: Vec<RequestSpec>) -> LoadProfile {
        LoadProfile::new(2, 8, requests)
    }

    #[tokio::test]
    async fn reports_totals_for_a_single_endpoint() {
        let profile = LoadProfile::new(1, 5, vec![RequestSpec::get("ok", "/ok")]);
        let report = run_load_test(test_router(), &profile).await.unwrap();

        assert_eq!(report.total_requests, 5);
        assert_eq!(report.successes, 5);
        assert_eq!(report.failures, 0);
        assert_eq!(report.status_counts.get("200"), Some(&5));
        assert_eq!(report.endpoints["ok"].count, 5);
        assert!(report.requests_per_second > 0.0);
    }

    #[tokio::test]
    async fn honors_weights_deterministically() {
        let profile = LoadProfile::new(
            1,
            8,
            vec![
                RequestSpec::get("ok", "/ok").with_weight(3),
                RequestSpec::get("missing", "/missing").with_weight(1),
            ],
        );
        let report = run_load_test(test_router(), &profile).await.unwrap();

        assert_eq!(report.endpoints["ok"].count, 6);
        assert_eq!(report.endpoints["missing"].count, 2);
    }

    #[tokio::test]
    async fn zero_weight_requests_are_never_sent() {
        let profile = LoadProfile::new(
            1,
            4,
            vec![
                RequestSpec::get("ok", "/ok").with_weight(1),
                RequestSpec::get("boom", "/boom").with_weight(0),
            ],
        );
        let report = run_load_test(test_router(), &profile).await.unwrap();

        assert_eq!(report.endpoints["ok"].count, 4);
        assert!(!report.endpoints.contains_key("boom"));
    }

    #[tokio::test]
    async fn error_statuses_count_as_failures_and_keep_latency_samples() {
        let profile = LoadProfile::new(
            1,
            4,
            vec![
                RequestSpec::get("ok", "/ok").with_weight(1),
                RequestSpec::get("boom", "/boom").with_weight(1),
            ],
        );
        let report = run_load_test(test_router(), &profile).await.unwrap();

        assert_eq!(report.total_requests, 4);
        assert_eq!(report.failures, 2);
        assert_eq!(report.successes, 2);
        assert_eq!(report.status_counts.get("500"), Some(&2));
        assert_eq!(report.endpoints["boom"].failures, 2);
        assert_eq!(report.endpoints["boom"].count, 2);
        assert_eq!(report.overall.count, 4);
    }

    #[tokio::test]
    async fn sends_json_bodies() {
        let profile = LoadProfile::new(
            1,
            2,
            vec![RequestSpec::post_json("echo", "/echo", "{\"a\":1}")],
        );
        let report = run_load_test(test_router(), &profile).await.unwrap();

        assert_eq!(report.failures, 0);
        assert_eq!(report.status_counts.get("200"), Some(&2));
    }

    #[tokio::test]
    async fn concurrency_above_one_sends_exactly_the_requested_volume() {
        let profile = LoadProfile::new(8, 40, vec![RequestSpec::get("ok", "/ok")]);
        let report = run_load_test(test_router(), &profile).await.unwrap();

        assert_eq!(report.total_requests, 40);
        assert_eq!(report.successes, 40);
        assert_eq!(report.concurrency, 8);
    }

    #[test]
    fn validate_rejects_zero_concurrency() {
        let mut profile = profile(vec![RequestSpec::get("ok", "/ok")]);
        profile.concurrency = 0;
        assert!(matches!(
            profile.validate(),
            Err(LoadTestError::ZeroConcurrency)
        ));
    }

    #[test]
    fn validate_rejects_zero_requests() {
        let mut profile = profile(vec![RequestSpec::get("ok", "/ok")]);
        profile.total_requests = 0;
        assert!(matches!(
            profile.validate(),
            Err(LoadTestError::ZeroRequests)
        ));
    }

    #[test]
    fn validate_rejects_an_empty_mix() {
        let profile = profile(vec![]);
        assert!(matches!(profile.validate(), Err(LoadTestError::EmptyMix)));
    }

    #[test]
    fn validate_rejects_an_all_zero_weight_mix() {
        let profile = profile(vec![RequestSpec::get("ok", "/ok").with_weight(0)]);
        assert!(matches!(
            profile.validate(),
            Err(LoadTestError::ZeroWeightMix)
        ));
    }

    #[test]
    fn validate_rejects_a_malformed_path() {
        let profile = profile(vec![RequestSpec::get("bad", "http://")]);
        assert!(matches!(
            profile.validate(),
            Err(LoadTestError::InvalidRequest { .. })
        ));
    }

    #[tokio::test]
    async fn invalid_profiles_do_not_run() {
        let profile = LoadProfile::new(0, 1, vec![RequestSpec::get("ok", "/ok")]);
        assert!(run_load_test(test_router(), &profile).await.is_err());
    }

    #[test]
    fn percentiles_use_nearest_rank() {
        let millis: Vec<f64> = (1..=100).map(|value| value as f64).collect();
        assert_eq!(percentile(&millis, 50.0), 50.0);
        assert_eq!(percentile(&millis, 95.0), 95.0);
        assert_eq!(percentile(&millis, 99.0), 99.0);
        assert_eq!(percentile(&millis, 100.0), 100.0);
        assert_eq!(percentile(&[], 50.0), 0.0);
    }

    #[tokio::test]
    async fn percentiles_are_monotonic() {
        let profile = LoadProfile::new(4, 32, vec![RequestSpec::get("ok", "/ok")]);
        let stats = run_load_test(test_router(), &profile)
            .await
            .unwrap()
            .overall;

        assert!(stats.p50_ms <= stats.p95_ms);
        assert!(stats.p95_ms <= stats.p99_ms);
        assert!(stats.p99_ms <= stats.max_ms);
    }

    #[tokio::test]
    async fn renders_a_text_report() {
        let profile = LoadProfile::new(1, 2, vec![RequestSpec::get("ok", "/ok")]);
        let text = run_load_test(test_router(), &profile)
            .await
            .unwrap()
            .render_text();

        assert!(text.contains("BeamRS load test"));
        assert!(text.contains("throughput"));
        assert!(text.contains("(all)"));
        assert!(text.contains("ok"));
    }
}
