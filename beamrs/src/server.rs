use std::{sync::Arc, time::Duration};

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use urlencoding::decode;

use crate::{
    app::BeamShell,
    config::Settings,
    domain::{Frequency, Prism, Ray, User},
    repository::BeamRepository,
};

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<BeamRepository>,
    #[allow(dead_code)]
    pub current_user: User,
}

pub async fn run() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let settings = Settings::from_env()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&settings.database_url)
        .await?;
    BeamRepository::initialize(&pool).await?;

    let repo = Arc::new(BeamRepository::new(pool.clone()));
    let current_user = repo.get_or_create_user("Anon1").await?;
    let state = AppState { repo, current_user };

    let app = Router::new()
        .route("/", get(home_page))
        .route("/health", get(health_check))
        .route(
            "/api/frequencies",
            get(list_frequencies).post(create_frequency),
        )
        .route(
            "/api/frequencies/:frequency_id/rays",
            get(list_rays_by_frequency),
        )
        .route("/api/users/get/:username", get(get_or_create_user))
        .route("/api/rays", post(create_ray))
        .route("/api/prisms", post(add_prism))
        .route("/api/prisms/:user_id/:ray_id", delete(remove_prism))
        .route("/api/rays/user/:username", get(list_rays_by_user))
        .route(
            "/api/rays/userprisms/:username",
            get(list_rays_prismed_by_user),
        )
        .route("/settings", get(settings_page))
        .route("/user/:username", get(user_page))
        .route("/frequency/:id", get(frequency_page))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", settings.host, settings.port)).await?;
    tracing::info!("BeamRS listening on {}:{}", settings.host, settings.port);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> impl IntoResponse {
    "ok"
}

async fn home_page() -> Html<String> {
    Html(render_page(
        "Home",
        "Welcome to BeamRS. Select a frequency or create one from the sidebar.",
    ))
}

async fn settings_page() -> Html<String> {
    Html(render_page(
        "Settings",
        "<form method=\"post\" action=\"/api/users/get/Anon1\"><input name=\"username\" placeholder=\"Username\" /><button>Update</button></form><div class=\"beam-demo\"><div class=\"beam-track\"><span class=\"beam-block\"></span></div><div class=\"beam-pass-counter\">Beam passes: <span>0</span></div></div>",
    ))
}

async fn user_page(
    Path(username): Path<String>,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let decoded = decode(&username).unwrap_or_else(|_| username.clone().into());
    let rays = state.repo.list_rays_by_user(decoded.as_ref()).await?;
    let body = format!(
        "<h2>User profile: {}</h2><ul>{}</ul>",
        html_escape(decoded.as_ref()),
        rays.iter()
            .map(|ray| format!("<li>\"{}\"</li>", html_escape(&ray.text)))
            .collect::<Vec<_>>()
            .join("")
    );
    Ok(Html(render_page("User", &body)))
}

async fn frequency_page(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let frequency_id: i32 = id.parse().unwrap_or(1);
    let frequencies = state.repo.list_frequencies().await?;
    let rays = state.repo.list_rays_by_frequency(frequency_id).await?;
    let body = format!(
        "<section class=\"frequency-page\"><h2>Frequency feed</h2><ul>{}</ul></section>",
        rays.iter()
            .map(|ray| format!(
                "<li><strong>{}</strong>: \"{}\" ({})</li>",
                html_escape(&ray.user_name),
                html_escape(&ray.text),
                ray.prism_count
            ))
            .collect::<Vec<_>>()
            .join("")
    );
    let sidebar = frequencies
        .iter()
        .map(|freq| {
            format!(
                "<a href=\"/frequency/{}\">{}</a>",
                freq.id,
                html_escape(&freq.name)
            )
        })
        .collect::<Vec<_>>()
        .join("<br>");
    Ok(Html(render_page_with_sidebar("Frequency", &body, &sidebar)))
}

async fn list_frequencies(State(state): State<AppState>) -> Result<Json<Vec<Frequency>>, AppError> {
    let items = state.repo.list_frequencies().await?;
    Ok(Json(items))
}

async fn create_frequency(
    State(state): State<AppState>,
    Json(payload): Json<Frequency>,
) -> Result<Json<Vec<Frequency>>, AppError> {
    state.repo.create_frequency(&payload.name).await?;
    let items = state.repo.list_frequencies().await?;
    Ok(Json(items))
}

async fn list_rays_by_frequency(
    Path(frequency_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<Ray>>, AppError> {
    let frequency_id: i32 = frequency_id.parse().unwrap_or(1);
    Ok(Json(state.repo.list_rays_by_frequency(frequency_id).await?))
}

async fn get_or_create_user(
    Path(username): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<User>, AppError> {
    let decoded = decode(&username).unwrap_or_else(|_| username.clone().into());
    Ok(Json(state.repo.get_or_create_user(decoded.as_ref()).await?))
}

async fn create_ray(
    State(state): State<AppState>,
    Json(payload): Json<Ray>,
) -> Result<Json<Vec<Ray>>, AppError> {
    let user_id = payload
        .user_id
        .ok_or_else(|| AppError::new("ray author is required"))?;
    let items = state
        .repo
        .create_ray(payload.frequency_id, user_id, &payload.text)
        .await?;
    Ok(Json(items))
}

async fn add_prism(
    State(state): State<AppState>,
    Json(payload): Json<Prism>,
) -> Result<Json<Vec<Ray>>, AppError> {
    let items = state
        .repo
        .add_prism(payload.user_id, payload.ray_id)
        .await?;
    Ok(Json(items))
}

async fn remove_prism(
    Path(path): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<Json<Vec<Ray>>, AppError> {
    let (user_id, ray_id) = path;
    let user_id: i32 = user_id.parse().unwrap_or_default();
    let ray_id: i32 = ray_id.parse().unwrap_or_default();
    let items = state.repo.remove_prism(user_id, ray_id).await?;
    Ok(Json(items))
}

async fn list_rays_by_user(
    Path(username): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<Ray>>, AppError> {
    let decoded = decode(&username).unwrap_or_else(|_| username.clone().into());
    Ok(Json(state.repo.list_rays_by_user(decoded.as_ref()).await?))
}

async fn list_rays_prismed_by_user(
    Path(username): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<Ray>>, AppError> {
    let decoded = decode(&username).unwrap_or_else(|_| username.clone().into());
    Ok(Json(
        state
            .repo
            .list_rays_prismed_by_user(decoded.as_ref())
            .await?,
    ))
}

fn render_page(title: &str, body: &str) -> String {
    let app_html = leptos::ssr::render_to_string(BeamShell);
    format!(
        r#"<!doctype html>
        <html lang="en">
        <head>
            <meta charset="utf-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1" />
            <title>{title}</title>
            <style>{}</style>
        </head>
        <body>
            {app_html}
            <main class="page-body">{body}</main>
        </body>
        </html>"#,
        STYLE,
    )
}

fn render_page_with_sidebar(title: &str, body: &str, sidebar: &str) -> String {
    format!(
        r#"<!doctype html>
        <html lang="en">
        <head>
            <meta charset="utf-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1" />
            <title>{title}</title>
            <style>{}</style>
        </head>
        <body>
            <div class="beam-shell">
                <aside class="sidebar">{sidebar}</aside>
                <main class="page-body">{body}</main>
            </div>
        </body>
        </html>"#,
        STYLE,
    )
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[derive(Debug)]
pub struct AppError {
    message: String,
}

impl AppError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.message).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        Self::new(value.to_string())
    }
}

const STYLE: &str = r#"
:root { color-scheme: light; font-family: sans-serif; }
body { margin: 0; background: #f6f5f2; color: #1f1b18; }
.beam-shell { display: flex; min-height: 100vh; }
.hero { display: flex; justify-content: space-between; align-items: center; padding: 1rem 1.5rem; background: #1b1d22; color: #f3f4f6; }
.brand { font-size: 2rem; font-weight: 800; letter-spacing: 0.22rem; }
.tagline { margin-left: 0.75rem; }
.main-nav a { color: #f3f4f6; margin-left: 1rem; }
.sidebar { width: 260px; background: #ebe3d6; padding: 1rem; border-right: 1px solid #d6c9b9; }
.page-body { padding: 2rem; flex: 1; }
.beam-demo { margin-top: 1rem; width: 300px; }
.beam-track { position: relative; height: 60px; background: #faf8f3; border: 1px solid #d7d1c9; overflow: hidden; }
.beam-block { position: absolute; left: -25px; top: 14px; width: 22px; height: 30px; background: #e11d48; box-shadow: 0 0 24px rgba(225, 29, 72, 0.8); animation: sweep 2.8s linear infinite; }
@keyframes sweep {
  0% { transform: translateX(0); opacity: 0.2; }
  20% { opacity: 1; }
  60% { opacity: 1; }
  100% { transform: translateX(320px); opacity: 0.2; }
}
.beam-pass-counter { margin-top: 0.75rem; }
* { box-sizing: border-box; }
"#;
