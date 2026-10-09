//! Load testing for the BeamRS application router.
//!
//! Two transports are supported:
//!
//! - [`Transport::InProcess`] drives an [`axum::Router`] directly through
//!   [`tower::ServiceExt::oneshot`] without opening sockets. That exercises the
//!   real extractors, handlers, error mapping, and [`BeamStore`] calls while
//!   excluding kernel networking and HTTP wire parsing, so the reported latency
//!   is application latency.
//! - [`Transport::Http`] sends HTTP/1.1 requests over TCP with a pooled
//!   keep-alive client to an [`HttpTarget`], either a running server or a
//!   router served on loopback by [`serve_loopback`]. The reported latency then
//!   includes connection handling, HTTP parsing, and server middleware.
//!
//! [`BeamStore`]: crate::repository::BeamStore

use std::{
    collections::BTreeMap,
    net::{Ipv4Addr, SocketAddr},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use axum::{
    body::{to_bytes, Body},
    http::{header::CONTENT_TYPE, uri::Uri, Method, Request},
    Router,
};
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use serde::Serialize;
use thiserror::Error;
use tokio::{net::TcpListener, task::JoinSet};
use tower::ServiceExt;

/// Responses at or above this status are counted as failures.
const FAILURE_STATUS: u16 = 400;
pub const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(30);

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
    #[error("target '{url}' is not valid: {reason}")]
    InvalidTarget { url: String, reason: String },
}

/// Base URL of an HTTP server to load, such as `http://127.0.0.1:8080`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpTarget {
    base_url: String,
    timeout: Duration,
}

impl HttpTarget {
    /// Accept an `http://host[:port]` URL with no path, query, or fragment.
    /// HTTPS is rejected because the client has no TLS connector.
    pub fn parse(url: &str) -> Result<Self, LoadTestError> {
        let invalid = |reason: &str| LoadTestError::InvalidTarget {
            url: url.to_string(),
            reason: reason.to_string(),
        };
        let uri: Uri = url
            .parse()
            .map_err(|_| invalid("expected a URL like http://127.0.0.1:8080"))?;
        match uri.scheme_str() {
            Some("http") => {}
            Some("https") => return Err(invalid("https is not supported; use http")),
            _ => return Err(invalid("the URL must start with http://")),
        }
        let authority = uri
            .authority()
            .ok_or_else(|| invalid("the URL must include a host"))?;
        if !matches!(uri.path(), "" | "/") || uri.query().is_some() || url.contains('#') {
            return Err(invalid(
                "the URL must not include a path, query, or fragment",
            ));
        }
        Ok(Self {
            base_url: format!("http://{authority}"),
            timeout: DEFAULT_HTTP_TIMEOUT,
        })
    }

    pub fn from_addr(addr: SocketAddr) -> Self {
        Self {
            base_url: format!("http://{addr}"),
            timeout: DEFAULT_HTTP_TIMEOUT,
        }
    }

    /// Set the deadline for the complete HTTP request and response body read.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

/// How requests reach the application.
#[derive(Clone, Debug)]
pub enum Transport {
    InProcess(Router),
    Http(HttpTarget),
}

impl Transport {
    fn label(&self) -> &'static str {
        match self {
            Self::InProcess(_) => "in-process",
            Self::Http(_) => "http",
        }
    }

    fn uri_prefix(&self) -> &str {
        match self {
            Self::InProcess(_) => "",
            Self::Http(target) => target.base_url(),
        }
    }

    fn request_timeout(&self) -> Option<Duration> {
        match self {
            Self::InProcess(_) => None,
            Self::Http(target) => Some(target.timeout()),
        }
    }
}

/// Serve `router` on an ephemeral loopback port so it can be loaded over HTTP.
///
/// The server runs until the returned task is aborted.
pub async fn serve_loopback(
    router: Router,
) -> std::io::Result<(HttpTarget, tokio::task::JoinHandle<()>)> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    let addr = listener.local_addr()?;
    let server = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, router).await {
            eprintln!("loopback server stopped: {error}");
        }
    });
    Ok((HttpTarget::from_addr(addr), server))
}

/// A per-run sender shared by all workers.
#[derive(Clone)]
enum Sender {
    Router(Router),
    Http(Client<HttpConnector, Body>),
}

struct Outcome {
    status: Option<u16>,
    body_read_error: bool,
    transport_error: bool,
}

impl Sender {
    fn new(transport: &Transport, concurrency: usize) -> Self {
        match transport {
            Transport::InProcess(router) => Self::Router(router.clone()),
            Transport::Http(_) => {
                let mut connector = HttpConnector::new();
                connector.set_nodelay(true);
                Self::Http(
                    Client::builder(TokioExecutor::new())
                        .pool_max_idle_per_host(concurrency)
                        .build(connector),
                )
            }
        }
    }

    async fn send(&self, request: Request<Body>) -> Outcome {
        let response = match self {
            Self::Router(router) => router.clone().oneshot(request).await.ok(),
            Self::Http(client) => client
                .request(request)
                .await
                .ok()
                .map(|response| response.map(Body::new)),
        };
        match response {
            Some(response) => {
                let status = response.status().as_u16();
                // Drain the body so the measurement covers the full response.
                let body_read_error = to_bytes(response.into_body(), usize::MAX).await.is_err();
                Outcome {
                    status: Some(status),
                    body_read_error,
                    transport_error: false,
                }
            }
            None => Outcome {
                status: None,
                body_read_error: false,
                transport_error: true,
            },
        }
    }
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

    fn build(&self, uri_prefix: &str) -> Result<Request<Body>, LoadTestError> {
        let builder = Request::builder()
            .method(self.method.clone())
            .uri(format!("{uri_prefix}{}", self.path));
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
            spec.build("")?;
        }
        Ok(())
    }

    /// Expand weights into a deterministic schedule of request indices.
    fn schedule(&self, max_entries: usize) -> Vec<usize> {
        let mut schedule = Vec::new();
        for (index, spec) in self.requests.iter().enumerate() {
            let remaining = max_entries.saturating_sub(schedule.len());
            if remaining == 0 {
                break;
            }
            for _ in 0..(spec.weight as usize).min(remaining) {
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
    body_read_error: bool,
    transport_error: bool,
    latency: Duration,
}

impl Sample {
    fn failed(&self) -> bool {
        if self.body_read_error {
            return true;
        }
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
    pub transport: String,
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
    fn from_samples(
        transport: &str,
        concurrency: usize,
        elapsed: Duration,
        samples: Vec<Sample>,
    ) -> Self {
        let borrowed: Vec<&Sample> = samples.iter().collect();
        let overall = LatencyStats::from_samples(&borrowed);

        let mut status_counts: BTreeMap<String, usize> = BTreeMap::new();
        for sample in &samples {
            let key = match sample.status {
                Some(status) => status.to_string(),
                None if sample.transport_error => "transport-error".to_string(),
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
            transport: transport.to_string(),
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
            "  transport        {}\n  concurrency      {}\n  requests         {}\n  elapsed          {:.1} ms\n  throughput       {:.1} req/s\n  successes        {}\n  failures         {}\n",
            self.transport,
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

/// Drive `router` in-process with `profile` and aggregate the results.
///
/// Equivalent to [`run_load_test_with`] using [`Transport::InProcess`].
pub async fn run_load_test(
    router: Router,
    profile: &LoadProfile,
) -> Result<LoadReport, LoadTestError> {
    run_load_test_with(&Transport::InProcess(router), profile).await
}

/// Drive `transport` with `profile` and aggregate the results.
///
/// Each of `profile.concurrency` workers claims request slots from a shared
/// counter until `profile.total_requests` is exhausted, so exactly that many
/// requests are sent regardless of how unevenly they complete. Requests that
/// cannot be delivered (for example, connection refused) count as failures
/// under the `transport-error` status key.
pub async fn run_load_test_with(
    transport: &Transport,
    profile: &LoadProfile,
) -> Result<LoadReport, LoadTestError> {
    profile.validate()?;
    let uri_prefix: Arc<str> = Arc::from(transport.uri_prefix());
    for spec in &profile.requests {
        spec.build(&uri_prefix)?;
    }

    let schedule = Arc::new(profile.schedule(profile.total_requests));
    let specs = Arc::new(profile.requests.clone());
    let cursor = Arc::new(AtomicUsize::new(0));
    let total_requests = profile.total_requests;
    let sender = Sender::new(transport, profile.concurrency);
    let request_timeout = transport.request_timeout();

    let started = Instant::now();
    let mut workers = JoinSet::new();
    for _ in 0..profile.concurrency {
        let sender = sender.clone();
        let uri_prefix = Arc::clone(&uri_prefix);
        let schedule = Arc::clone(&schedule);
        let specs = Arc::clone(&specs);
        let cursor = Arc::clone(&cursor);
        workers.spawn(async move {
            let mut samples = Vec::new();
            loop {
                let slot = cursor.fetch_add(1, Ordering::Relaxed);
                if slot >= total_requests {
                    break;
                }
                let spec = &specs[schedule[slot % schedule.len()]];
                let request = match spec.build(&uri_prefix) {
                    Ok(request) => request,
                    Err(_) => {
                        samples.push(Sample {
                            name: spec.name.clone(),
                            status: None,
                            body_read_error: false,
                            transport_error: false,
                            latency: Duration::ZERO,
                        });
                        continue;
                    }
                };
                let call_started = Instant::now();
                let outcome = match request_timeout {
                    Some(timeout) => {
                        match tokio::time::timeout(timeout, sender.send(request)).await {
                            Ok(outcome) => outcome,
                            Err(_) => Outcome {
                                status: None,
                                body_read_error: false,
                                transport_error: true,
                            },
                        }
                    }
                    None => sender.send(request).await,
                };
                samples.push(Sample {
                    name: spec.name.clone(),
                    status: outcome.status,
                    body_read_error: outcome.body_read_error,
                    transport_error: outcome.transport_error,
                    latency: call_started.elapsed(),
                });
            }
            samples
        });
    }

    let mut samples = Vec::with_capacity(total_requests);
    while let Some(result) = workers.join_next().await {
        match result {
            Ok(worker_samples) => samples.extend(worker_samples),
            Err(error) => {
                workers.abort_all();
                while workers.join_next().await.is_some() {}
                return Err(LoadTestError::Worker(error.to_string()));
            }
        }
    }
    let elapsed = started.elapsed();

    Ok(LoadReport::from_samples(
        transport.label(),
        profile.concurrency,
        elapsed,
        samples,
    ))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Bytes,
        http::StatusCode,
        response::Response,
        routing::{get, post},
        Router,
    };
    use futures_util::StreamExt;

    use super::*;

    fn test_router() -> Router {
        Router::new()
            .route("/ok", get(|| async { StatusCode::OK }))
            .route("/missing", get(|| async { StatusCode::NOT_FOUND }))
            .route("/boom", get(|| async { StatusCode::INTERNAL_SERVER_ERROR }))
            .route("/echo", post(|body: String| async move { body }))
    }

    fn slow_headers_router() -> Router {
        Router::new().route(
            "/slow-headers",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(1)).await;
                StatusCode::OK
            }),
        )
    }

    fn slow_body_router() -> Router {
        Router::new().route(
            "/slow-body",
            get(|| async {
                let body = futures_util::stream::once(async {
                    Ok::<_, std::io::Error>(Bytes::from_static(b"partial"))
                })
                .chain(futures_util::stream::pending());
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from_stream(body))
                    .unwrap()
            }),
        )
    }

    fn failing_body_router() -> Router {
        Router::new().route(
            "/body-error",
            get(|| async {
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from_stream(futures_util::stream::once(async {
                        Err::<Bytes, _>(std::io::Error::other("body read failed"))
                    })))
                    .unwrap()
            }),
        )
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
    async fn response_body_read_errors_count_as_failures_without_losing_status() {
        let profile = LoadProfile::new(1, 1, vec![RequestSpec::get("body-error", "/body-error")]);
        let report = run_load_test(failing_body_router(), &profile)
            .await
            .unwrap();

        assert_eq!(report.failures, 1);
        assert_eq!(report.successes, 0);
        assert_eq!(report.status_counts.get("200"), Some(&1));
        assert_eq!(report.endpoints["body-error"].failures, 1);
        assert_eq!(report.endpoints["body-error"].count, 1);
    }

    #[test]
    fn schedule_expansion_is_bounded_by_the_request_volume() {
        let profile = LoadProfile::new(
            1,
            3,
            vec![RequestSpec::get("ok", "/ok").with_weight(u32::MAX)],
        );

        assert_eq!(profile.schedule(profile.total_requests), vec![0, 0, 0]);
    }

    #[tokio::test]
    async fn concurrency_above_one_sends_exactly_the_requested_volume() {
        let profile = LoadProfile::new(8, 40, vec![RequestSpec::get("ok", "/ok")]);
        let report = run_load_test(test_router(), &profile).await.unwrap();

        assert_eq!(report.total_requests, 40);
        assert_eq!(report.successes, 40);
        assert_eq!(report.concurrency, 8);
    }

    #[tokio::test]
    async fn worker_panic_aborts_and_awaits_other_workers() {
        let requests_started = Arc::new(AtomicUsize::new(0));
        let handler_requests = Arc::clone(&requests_started);
        let router = Router::new().route(
            "/panic-once",
            get(move || {
                let requests_started = Arc::clone(&handler_requests);
                async move {
                    if requests_started.fetch_add(1, Ordering::SeqCst) == 0 {
                        panic!("intentional worker panic");
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    StatusCode::OK
                }
            }),
        );
        let profile = LoadProfile::new(4, 100, vec![RequestSpec::get("panic", "/panic-once")]);

        assert!(matches!(
            run_load_test(router, &profile).await,
            Err(LoadTestError::Worker(_))
        ));
        let requests_at_return = requests_started.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(requests_started.load(Ordering::SeqCst), requests_at_return);
        assert!(requests_at_return < profile.total_requests);
    }

    #[tokio::test]
    async fn cancelling_run_aborts_all_workers() {
        let requests_started = Arc::new(AtomicUsize::new(0));
        let handler_requests = Arc::clone(&requests_started);
        let router = Router::new().route(
            "/slow",
            get(move || {
                let requests_started = Arc::clone(&handler_requests);
                async move {
                    requests_started.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    StatusCode::OK
                }
            }),
        );
        let profile = LoadProfile::new(4, 1_000, vec![RequestSpec::get("slow", "/slow")]);
        let run = tokio::spawn(async move { run_load_test(router, &profile).await });
        tokio::time::timeout(Duration::from_secs(1), async {
            while requests_started.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();

        run.abort();
        let _ = run.await;
        let requests_at_cancel = requests_started.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(requests_started.load(Ordering::SeqCst), requests_at_cancel);
        assert!(requests_at_cancel < 1_000);
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

    #[tokio::test]
    async fn http_transport_round_trips_through_a_loopback_server() {
        let (target, server) = serve_loopback(test_router()).await.unwrap();
        let profile = LoadProfile::new(
            4,
            20,
            vec![
                RequestSpec::get("ok", "/ok").with_weight(1),
                RequestSpec::get("missing", "/missing").with_weight(1),
            ],
        );
        let report = run_load_test_with(&Transport::Http(target), &profile)
            .await
            .unwrap();
        server.abort();

        assert_eq!(report.transport, "http");
        assert_eq!(report.total_requests, 20);
        assert_eq!(report.status_counts.get("200"), Some(&10));
        assert_eq!(report.status_counts.get("404"), Some(&10));
        assert_eq!(report.failures, 10);
    }

    #[tokio::test]
    async fn http_transport_sends_json_bodies() {
        let (target, server) = serve_loopback(test_router()).await.unwrap();
        let profile = LoadProfile::new(
            2,
            4,
            vec![RequestSpec::post_json("echo", "/echo", "{\"a\":1}")],
        );
        let report = run_load_test_with(&Transport::Http(target), &profile)
            .await
            .unwrap();
        server.abort();

        assert_eq!(report.failures, 0);
        assert_eq!(report.status_counts.get("200"), Some(&4));
    }

    #[tokio::test]
    async fn unreachable_http_targets_count_as_transport_failures() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let profile = LoadProfile::new(2, 6, vec![RequestSpec::get("ok", "/ok")]);
        let report = run_load_test_with(&Transport::Http(HttpTarget::from_addr(addr)), &profile)
            .await
            .unwrap();

        assert_eq!(report.total_requests, 6);
        assert_eq!(report.failures, 6);
        assert_eq!(report.status_counts.get("transport-error"), Some(&6));
    }

    #[tokio::test]
    async fn http_timeout_covers_waiting_for_response_headers() {
        let (target, server) = serve_loopback(slow_headers_router()).await.unwrap();
        let target = target.with_timeout(Duration::from_millis(30));
        let profile = LoadProfile::new(1, 1, vec![RequestSpec::get("slow", "/slow-headers")]);

        let report = run_load_test_with(&Transport::Http(target), &profile)
            .await
            .unwrap();
        server.abort();

        assert_eq!(report.failures, 1);
        assert_eq!(report.status_counts.get("transport-error"), Some(&1));
        assert!(report.elapsed_ms < 500.0);
    }

    #[tokio::test]
    async fn http_timeout_covers_response_body_drain() {
        let (target, server) = serve_loopback(slow_body_router()).await.unwrap();
        let target = target.with_timeout(Duration::from_millis(30));
        let profile = LoadProfile::new(1, 1, vec![RequestSpec::get("slow", "/slow-body")]);

        let report = run_load_test_with(&Transport::Http(target), &profile)
            .await
            .unwrap();
        server.abort();

        assert_eq!(report.failures, 1);
        assert_eq!(report.status_counts.get("transport-error"), Some(&1));
        assert!(report.elapsed_ms < 500.0);
    }

    #[test]
    fn http_targets_are_validated() {
        let target = HttpTarget::parse("http://127.0.0.1:8080").unwrap();
        assert_eq!(target.base_url(), "http://127.0.0.1:8080");
        assert_eq!(target.timeout(), DEFAULT_HTTP_TIMEOUT);
        assert_eq!(
            target
                .clone()
                .with_timeout(Duration::from_secs(5))
                .timeout(),
            Duration::from_secs(5)
        );
        assert_eq!(
            HttpTarget::parse("http://localhost:3000/")
                .unwrap()
                .base_url(),
            "http://localhost:3000"
        );
        for invalid in [
            "https://localhost:3000",
            "localhost:3000",
            "http://localhost:3000/api",
            "http://localhost:3000?x=1",
            "not a url",
        ] {
            assert!(
                matches!(
                    HttpTarget::parse(invalid),
                    Err(LoadTestError::InvalidTarget { .. })
                ),
                "{invalid} should be rejected"
            );
        }
    }

    #[tokio::test]
    async fn in_process_reports_are_labelled() {
        let profile = LoadProfile::new(1, 1, vec![RequestSpec::get("ok", "/ok")]);
        let report = run_load_test(test_router(), &profile).await.unwrap();

        assert_eq!(report.transport, "in-process");
        assert!(report.render_text().contains("transport        in-process"));
    }
}
