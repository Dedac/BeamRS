use std::{sync::Arc, time::Duration};

use anyhow::Result;
use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::Serialize;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use urlencoding::decode;

use crate::{
    app::{escape_html, render_page, render_ray_cards, ActivePage},
    config::Settings,
    domain::{
        Frequency, FrequencyInput, PrismInput, PrismResult, Ray, RayInput, User, UsernameInput,
    },
    repository::{BeamRepository, BeamStore, RepositoryError},
};

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<dyn BeamStore>,
}

pub async fn run() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "beamrs=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let settings = Settings::from_env()?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&settings.database_url)
        .await?;
    BeamRepository::initialize(&pool).await?;

    let state = AppState {
        repo: Arc::new(BeamRepository::new(pool)),
    };
    let app = router(state);
    let address = format!("{}:{}", settings.host, settings.port);
    let listener = tokio::net::TcpListener::bind(&address).await?;
    tracing::info!("BeamRS listening on http://{address}");
    axum::serve(listener, app).await?;
    Ok(())
}

pub fn router(state: AppState) -> Router {
    Router::new()
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
        .route("/api/users", post(get_or_create_user))
        .route("/api/users/:user_id", patch(update_user_name))
        .route("/api/users/:username/rays", get(list_rays_by_user))
        .route(
            "/api/users/:username/prisms",
            get(list_rays_prismed_by_user),
        )
        .route("/api/rays", post(create_ray))
        .route("/api/prisms", post(add_prism))
        .route("/api/prisms/:user_id/:ray_id", delete(remove_prism))
        .route("/settings", get(settings_page))
        .route("/user/:username", get(user_page))
        .route("/frequency/:frequency_id", get(frequency_page))
        .nest_service(
            "/static",
            ServeDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/static")),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_check() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn home_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let frequencies = state.repo.list_frequencies().await?;
    let content = r##"
        <section class="welcome-panel">
            <div class="eyebrow">Rust-powered transmission</div>
            <h1>Welcome to <span>BeamRS</span></h1>
            <p>Pick a frequency, send a ray, and prism the signals that deserve a wider spectrum.</p>
            <div class="welcome-actions">
                <a class="primary-button" href="#frequency-form">Create a frequency</a>
                <a class="text-link" href="/settings">Tune your identity →</a>
            </div>
        </section>
        <section class="explainer-grid" aria-label="How BeamRS works">
            <article><strong>01</strong><h2>Tune in</h2><p>Frequencies keep every conversation on wavelength.</p></article>
            <article><strong>02</strong><h2>Send a ray</h2><p>Share a focused thought in 300 characters or fewer.</p></article>
            <article><strong>03</strong><h2>Prism it</h2><p>Refract a great ray to show that it resonated.</p></article>
        </section>
    "##;
    Ok(Html(render_page(
        "Home",
        ActivePage::Home,
        frequencies,
        content.to_string(),
    )))
}

async fn settings_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let frequencies = state.repo.list_frequencies().await?;
    let content = r#"
        <section class="page-heading">
            <div><span class="eyebrow">Signal controls</span><h1>Settings</h1></div>
            <p>Your identity is stored in this browser. BeamRS has no authentication, matching the original Beam experience.</p>
        </section>
        <div class="settings-grid">
            <section class="panel">
                <h2>Display name</h2>
                <p>Use letters, numbers, dashes, or underscores. Changing this name updates your existing profile.</p>
                <form id="username-form" class="stacked-form">
                    <label for="username-input">Username</label>
                    <input id="username-input" name="username" maxlength="40" autocomplete="nickname" required />
                    <button class="primary-button" type="submit">Update identity</button>
                    <p class="form-status" role="status"></p>
                </form>
            </section>
            <section class="panel beam-lab">
                <div>
                    <span class="eyebrow">Optics lab</span>
                    <h2>Beam pass counter</h2>
                    <p>A tiny Rust-inspired signal, rendered with the browser canvas.</p>
                </div>
                <canvas id="beam-canvas" width="640" height="220" aria-label="Animated red beam traveling across an optics track"></canvas>
                <div class="counter"><strong id="beam-pass-count">0</strong><span>completed passes</span></div>
            </section>
        </div>
    "#;
    Ok(Html(render_page(
        "Settings",
        ActivePage::Settings,
        frequencies,
        content.to_string(),
    )))
}

async fn user_page(
    Path(username): Path<String>,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let username = decode_path(&username)?;
    let frequencies = state.repo.list_frequencies().await?;
    let authored = state.repo.list_rays_by_user(&username).await?;
    let prismed = state.repo.list_rays_prismed_by_user(&username).await?;
    let content = format!(
        r#"
        <section class="profile-header">
            <div class="avatar" aria-hidden="true">{initial}</div>
            <div><span class="eyebrow">Beam profile</span><h1>@{username}</h1><p>{authored_count} authored rays · {prismed_count} prismed rays</p></div>
        </section>
        <section class="feed-section">
            <div class="section-title"><h2>Authored rays</h2><span>{authored_count}</span></div>
            <div class="ray-list">{authored}</div>
        </section>
        <section class="feed-section">
            <div class="section-title"><h2>Prismed rays</h2><span>{prismed_count}</span></div>
            <div class="ray-list">{prismed}</div>
        </section>
        "#,
        initial = escape_html(&username.chars().next().unwrap_or('?').to_string()),
        username = escape_html(&username),
        authored_count = authored.len(),
        prismed_count = prismed.len(),
        authored = render_ray_cards(&authored, "No rays transmitted by this user yet."),
        prismed = render_ray_cards(&prismed, "No rays prismed by this user yet."),
    );
    Ok(Html(render_page(
        &format!("@{username}"),
        ActivePage::User,
        frequencies,
        content,
    )))
}

async fn frequency_page(
    Path(frequency_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let frequencies = state.repo.list_frequencies().await?;
    let frequency = frequencies
        .iter()
        .find(|frequency| frequency.id == frequency_id)
        .cloned()
        .ok_or_else(|| AppError::not_found(format!("frequency {frequency_id} not found")))?;
    let rays = state.repo.list_rays_by_frequency(frequency_id).await?;
    let content = format!(
        r#"
        <section class="page-heading frequency-heading">
            <div><span class="eyebrow">Frequency {id}</span><h1>#{name}</h1></div>
            <p>{count} rays currently traveling on this wavelength.</p>
        </section>
        <section class="composer-panel">
            <form id="ray-form" data-frequency-id="{id}">
                <label for="ray-text">Send a ray to #{name}</label>
                <textarea id="ray-text" name="text" maxlength="300" rows="3" placeholder="What is on your wavelength?" required></textarea>
                <div class="composer-footer">
                    <span><span id="ray-character-count">0</span>/300</span>
                    <button class="primary-button" type="submit">Transmit ray</button>
                </div>
                <p class="form-status" role="status"></p>
            </form>
        </section>
        <section class="feed-section">
            <div class="section-title"><h2>Latest rays</h2><span>{count}</span></div>
            <div class="ray-list">{ray_cards}</div>
        </section>
        "#,
        id = frequency.id,
        name = escape_html(&frequency.name),
        count = rays.len(),
        ray_cards = render_ray_cards(&rays, "This frequency is quiet. Send the first ray."),
    );
    Ok(Html(render_page(
        &format!("#{}", frequency.name),
        ActivePage::Frequency,
        frequencies,
        content,
    )))
}

async fn list_frequencies(State(state): State<AppState>) -> Result<Json<Vec<Frequency>>, AppError> {
    Ok(Json(state.repo.list_frequencies().await?))
}

async fn create_frequency(
    State(state): State<AppState>,
    payload: Result<Json<FrequencyInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Frequency>), AppError> {
    let payload = json_payload(payload)?;
    let frequency = state.repo.create_frequency(&payload.name).await?;
    Ok((StatusCode::CREATED, Json(frequency)))
}

async fn list_rays_by_frequency(
    Path(frequency_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<Vec<Ray>>, AppError> {
    Ok(Json(state.repo.list_rays_by_frequency(frequency_id).await?))
}

async fn get_or_create_user(
    State(state): State<AppState>,
    payload: Result<Json<UsernameInput>, JsonRejection>,
) -> Result<Json<User>, AppError> {
    let payload = json_payload(payload)?;
    Ok(Json(
        state.repo.get_or_create_user(&payload.username).await?,
    ))
}

async fn update_user_name(
    Path(user_id): Path<i32>,
    State(state): State<AppState>,
    payload: Result<Json<UsernameInput>, JsonRejection>,
) -> Result<Json<User>, AppError> {
    let payload = json_payload(payload)?;
    Ok(Json(
        state
            .repo
            .update_user_name(user_id, &payload.username)
            .await?,
    ))
}

async fn create_ray(
    State(state): State<AppState>,
    payload: Result<Json<RayInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Ray>), AppError> {
    let payload = json_payload(payload)?;
    let ray = state
        .repo
        .create_ray(payload.frequency_id, payload.user_id, &payload.text)
        .await?;
    Ok((StatusCode::CREATED, Json(ray)))
}

async fn add_prism(
    State(state): State<AppState>,
    payload: Result<Json<PrismInput>, JsonRejection>,
) -> Result<Json<PrismResult>, AppError> {
    let payload = json_payload(payload)?;
    Ok(Json(
        state
            .repo
            .add_prism(payload.user_id, payload.ray_id)
            .await?,
    ))
}

async fn remove_prism(
    Path((user_id, ray_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
) -> Result<Json<PrismResult>, AppError> {
    Ok(Json(state.repo.remove_prism(user_id, ray_id).await?))
}

async fn list_rays_by_user(
    Path(username): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<Ray>>, AppError> {
    let username = decode_path(&username)?;
    Ok(Json(state.repo.list_rays_by_user(&username).await?))
}

async fn list_rays_prismed_by_user(
    Path(username): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<Ray>>, AppError> {
    let username = decode_path(&username)?;
    Ok(Json(state.repo.list_rays_prismed_by_user(&username).await?))
}

fn decode_path(value: &str) -> Result<String, AppError> {
    decode(value)
        .map(|decoded| decoded.into_owned())
        .map_err(|_| AppError::bad_request("path contains invalid URL encoding"))
}

fn json_payload<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, AppError> {
    payload
        .map(|Json(value)| value)
        .map_err(|error| AppError::bad_request(error.body_text()))
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

impl From<RepositoryError> for AppError {
    fn from(value: RepositoryError) -> Self {
        match value {
            RepositoryError::Validation(message) => Self::bad_request(message),
            RepositoryError::NotFound(message) => Self::not_found(message),
            RepositoryError::Conflict(message) => Self {
                status: StatusCode::CONFLICT,
                message,
            },
            RepositoryError::Database(error) => {
                tracing::error!(?error, "repository operation failed");
                Self {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    message: "an internal database error occurred".to_string(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        sync::Mutex,
    };

    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{Method, Request},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;
    use crate::repository::RepositoryResult;

    #[derive(Default)]
    struct MemoryStore {
        inner: Mutex<MemoryData>,
    }

    #[derive(Default)]
    struct MemoryData {
        users: Vec<User>,
        frequencies: Vec<Frequency>,
        rays: Vec<Ray>,
        prisms: HashSet<(i32, i32)>,
    }

    impl MemoryStore {
        fn seeded() -> Self {
            Self {
                inner: Mutex::new(MemoryData {
                    users: vec![User {
                        id: 1,
                        username: "Anon1".to_string(),
                    }],
                    frequencies: vec![Frequency {
                        id: 1,
                        name: "general".to_string(),
                    }],
                    rays: vec![Ray {
                        id: 1,
                        frequency_id: 1,
                        text: "First light".to_string(),
                        user_id: Some(1),
                        user_name: "Anon1".to_string(),
                        prism_count: 0,
                        users_prismed: vec![],
                    }],
                    prisms: HashSet::new(),
                }),
            }
        }

        fn refresh_prisms(data: &mut MemoryData) {
            let usernames: HashMap<i32, String> = data
                .users
                .iter()
                .map(|user| (user.id, user.username.clone()))
                .collect();
            for ray in &mut data.rays {
                ray.users_prismed = data
                    .prisms
                    .iter()
                    .filter(|(_, ray_id)| *ray_id == ray.id)
                    .filter_map(|(user_id, _)| usernames.get(user_id).cloned())
                    .collect();
                ray.users_prismed.sort();
                ray.prism_count = ray.users_prismed.len() as i32;
            }
        }
    }

    #[async_trait]
    impl BeamStore for MemoryStore {
        async fn list_frequencies(&self) -> RepositoryResult<Vec<Frequency>> {
            Ok(self.inner.lock().unwrap().frequencies.clone())
        }

        async fn create_frequency(&self, name: &str) -> RepositoryResult<Frequency> {
            let name = crate::domain::validate_frequency_name(name)
                .map_err(RepositoryError::Validation)?;
            let mut data = self.inner.lock().unwrap();
            if data.frequencies.iter().any(|item| item.name == name) {
                return Err(RepositoryError::Conflict(
                    "frequency already exists".to_string(),
                ));
            }
            let frequency = Frequency {
                id: data.frequencies.len() as i32 + 1,
                name,
            };
            data.frequencies.push(frequency.clone());
            Ok(frequency)
        }

        async fn get_or_create_user(&self, username: &str) -> RepositoryResult<User> {
            let username =
                crate::domain::validate_username(username).map_err(RepositoryError::Validation)?;
            let mut data = self.inner.lock().unwrap();
            if let Some(user) = data.users.iter().find(|user| user.username == username) {
                return Ok(user.clone());
            }
            let user = User {
                id: data.users.len() as i32 + 1,
                username,
            };
            data.users.push(user.clone());
            Ok(user)
        }

        async fn update_user_name(&self, user_id: i32, username: &str) -> RepositoryResult<User> {
            let username =
                crate::domain::validate_username(username).map_err(RepositoryError::Validation)?;
            let mut data = self.inner.lock().unwrap();
            if data
                .users
                .iter()
                .any(|user| user.id != user_id && user.username == username)
            {
                return Err(RepositoryError::Conflict(
                    "username already exists".to_string(),
                ));
            }
            let user = data
                .users
                .iter_mut()
                .find(|user| user.id == user_id)
                .ok_or_else(|| RepositoryError::NotFound(format!("user {user_id}")))?;
            user.username = username;
            Ok(user.clone())
        }

        async fn list_rays_by_frequency(&self, frequency_id: i32) -> RepositoryResult<Vec<Ray>> {
            let mut rays: Vec<_> = self
                .inner
                .lock()
                .unwrap()
                .rays
                .iter()
                .filter(|ray| ray.frequency_id == frequency_id)
                .cloned()
                .collect();
            rays.reverse();
            Ok(rays)
        }

        async fn list_rays_by_user(&self, username: &str) -> RepositoryResult<Vec<Ray>> {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .rays
                .iter()
                .filter(|ray| ray.user_name == username)
                .cloned()
                .collect())
        }

        async fn list_rays_prismed_by_user(&self, username: &str) -> RepositoryResult<Vec<Ray>> {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .rays
                .iter()
                .filter(|ray| ray.users_prismed.iter().any(|user| user == username))
                .cloned()
                .collect())
        }

        async fn create_ray(
            &self,
            frequency_id: i32,
            user_id: i32,
            text: &str,
        ) -> RepositoryResult<Ray> {
            let text =
                crate::domain::validate_ray_text(text).map_err(RepositoryError::Validation)?;
            let mut data = self.inner.lock().unwrap();
            if !data
                .frequencies
                .iter()
                .any(|frequency| frequency.id == frequency_id)
            {
                return Err(RepositoryError::NotFound(format!(
                    "frequency {frequency_id}"
                )));
            }
            let username = data
                .users
                .iter()
                .find(|user| user.id == user_id)
                .map(|user| user.username.clone())
                .ok_or_else(|| RepositoryError::NotFound(format!("user {user_id}")))?;
            let ray = Ray {
                id: data.rays.len() as i32 + 1,
                frequency_id,
                text,
                user_id: Some(user_id),
                user_name: username,
                prism_count: 0,
                users_prismed: vec![],
            };
            data.rays.push(ray.clone());
            Ok(ray)
        }

        async fn add_prism(&self, user_id: i32, ray_id: i32) -> RepositoryResult<PrismResult> {
            let mut data = self.inner.lock().unwrap();
            if !data.users.iter().any(|user| user.id == user_id) {
                return Err(RepositoryError::NotFound(format!("user {user_id}")));
            }
            if !data.rays.iter().any(|ray| ray.id == ray_id) {
                return Err(RepositoryError::NotFound(format!("ray {ray_id}")));
            }
            data.prisms.insert((user_id, ray_id));
            Self::refresh_prisms(&mut data);
            let count = data
                .rays
                .iter()
                .find(|ray| ray.id == ray_id)
                .unwrap()
                .prism_count;
            Ok(PrismResult {
                ray_id,
                prism_count: count,
                prismed: true,
            })
        }

        async fn remove_prism(&self, user_id: i32, ray_id: i32) -> RepositoryResult<PrismResult> {
            let mut data = self.inner.lock().unwrap();
            if !data.rays.iter().any(|ray| ray.id == ray_id) {
                return Err(RepositoryError::NotFound(format!("ray {ray_id}")));
            }
            data.prisms.remove(&(user_id, ray_id));
            Self::refresh_prisms(&mut data);
            let count = data
                .rays
                .iter()
                .find(|ray| ray.id == ray_id)
                .unwrap()
                .prism_count;
            Ok(PrismResult {
                ray_id,
                prism_count: count,
                prismed: false,
            })
        }
    }

    fn test_app() -> Router {
        router(AppState {
            repo: Arc::new(MemoryStore::seeded()),
        })
    }

    async fn json_request(
        app: &Router,
        method: Method,
        uri: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, value)
    }

    #[tokio::test]
    async fn user_creation_requires_post_and_is_idempotent() {
        let app = test_app();
        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/users/get/Anon1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_response.status(), StatusCode::NOT_FOUND);

        let first = json_request(
            &app,
            Method::POST,
            "/api/users",
            serde_json::json!({"username": "new_user"}),
        )
        .await;
        let second = json_request(
            &app,
            Method::POST,
            "/api/users",
            serde_json::json!({"username": "new_user"}),
        )
        .await;
        assert_eq!(first.0, StatusCode::OK);
        assert_eq!(first.1, second.1);
    }

    #[tokio::test]
    async fn invalid_frequency_returns_bad_request_and_duplicate_returns_conflict() {
        let app = test_app();
        let invalid = json_request(
            &app,
            Method::POST,
            "/api/frequencies",
            serde_json::json!({"name": "   "}),
        )
        .await;
        assert_eq!(invalid.0, StatusCode::BAD_REQUEST);
        assert_eq!(
            invalid.1["error"],
            serde_json::json!("frequency name must not be blank")
        );

        let duplicate = json_request(
            &app,
            Method::POST,
            "/api/frequencies",
            serde_json::json!({"name": "general"}),
        )
        .await;
        assert_eq!(duplicate.0, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn prism_add_and_remove_are_idempotent() {
        let app = test_app();
        for expected_count in [1, 1] {
            let (status, body) = json_request(
                &app,
                Method::POST,
                "/api/prisms",
                serde_json::json!({"user_id": 1, "ray_id": 1}),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body["prism_count"], expected_count);
            assert_eq!(body["prismed"], true);
        }

        for expected_count in [0, 0] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(Method::DELETE)
                        .uri("/api/prisms/1/1")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let status = response.status();
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(status, StatusCode::OK);
            assert_eq!(body["prism_count"], expected_count);
            assert_eq!(body["prismed"], false);
        }
    }

    #[tokio::test]
    async fn core_user_flow_creates_and_lists_a_ray() {
        let app = test_app();
        let created = json_request(
            &app,
            Method::POST,
            "/api/rays",
            serde_json::json!({
                "frequency_id": 1,
                "user_id": 1,
                "text": "  A new wavelength  "
            }),
        )
        .await;
        assert_eq!(created.0, StatusCode::CREATED);
        assert_eq!(created.1["text"], "A new wavelength");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/frequencies/1/rays")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let rays: Vec<Ray> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(rays.len(), 2);
        assert_eq!(rays[0].text, "A new wavelength");
    }

    #[tokio::test]
    async fn malformed_json_uses_the_api_error_shape() {
        let app = test_app();
        let (status, body) = json_request(
            &app,
            Method::POST,
            "/api/rays",
            serde_json::json!({"frequency_id": 1, "text": "missing user"}),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"]
            .as_str()
            .is_some_and(|message| message.contains("missing field `user_id`")));
    }
}
