//! Deployment-owned developer-token service for MusicKit.
//!
//! The service is intentionally tiny: it holds the Apple `.p8` key, mints
//! short-lived ES256 developer tokens, and never sees a user's Music User Token.

use std::{
    env,
    error::Error,
    io,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sosumi_core::apple::{DeveloperTokenSigner, MAX_TOKEN_TTL};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    signer: Arc<DeveloperTokenSigner>,
}

#[derive(Clone)]
struct AuthState {
    api_key: String,
}

#[derive(Debug, Default, Deserialize)]
struct TokenRequest {
    /// Optional web origins to bind into the JWT `origin` claim.
    #[serde(default)]
    origin: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct TokenResponse {
    token: String,
    expires_at: u64,
    expires_in: u64,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let team_id = required_env("APPLE_TEAM_ID")?;
    let key_id = required_env("APPLE_KEY_ID")?;
    let pem = read_private_key()?;
    let ttl_seconds = parse_ttl()?;
    let bind = bind_address()?;
    let api_key = env::var("SOSUMI_TOKEN_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let origins = env::var("SOSUMI_TOKEN_ORIGINS")
        .ok()
        .filter(|value| !value.trim().is_empty());

    let signer = DeveloperTokenSigner::from_pkcs8_pem(
        &pem,
        team_id,
        key_id,
        Duration::from_secs(ttl_seconds),
    )?;
    let state = Arc::new(AppState {
        signer: Arc::new(signer),
    });

    let mut protected = Router::new().route("/v1/developer-token", post(issue_token));
    if let Some(api_key) = api_key {
        protected = protected.layer(middleware::from_fn_with_state(
            AuthState { api_key },
            require_api_key,
        ));
    }

    let mut app = Router::new()
        .route("/health", get(health))
        .merge(protected)
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    app = app.layer(cors_layer(origins.as_deref())?);

    let listener = tokio::net::TcpListener::bind(bind).await?;
    info!(%bind, ttl_seconds, "sosumi token service listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn issue_token(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TokenRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let now = unix_now();
    let origin = request.origin.filter(|origins| !origins.is_empty());
    let token = state
        .signer
        .issue_at_with_origin(now, origin)
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("could not issue developer token: {error}"),
            )
        })?;
    let claims = token.claims();

    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(TokenResponse {
            token: token.as_str().to_owned(),
            expires_at: claims.exp,
            expires_in: claims.exp.saturating_sub(now),
        }),
    ))
}

async fn require_api_key(
    State(auth): State<AuthState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let supplied = headers
        .get("x-sosumi-key")
        .and_then(|value| value.to_str().ok());

    if supplied != Some(auth.api_key.as_str()) {
        return (StatusCode::UNAUTHORIZED, "invalid token-service key").into_response();
    }

    next.run(request).await
}

fn cors_layer(origins: Option<&str>) -> Result<CorsLayer, Box<dyn Error>> {
    let Some(origins) = origins else {
        return Ok(CorsLayer::new());
    };

    if origins.trim() == "*" {
        warn!("token service CORS is permissive");
        return Ok(CorsLayer::permissive());
    }

    let origins = origins
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(HeaderValue::from_str)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            HeaderName::from_static("x-sosumi-key"),
        ]))
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sosumi_token=info,tower_http=info".into()),
        )
        .init();
}

fn required_env(key: &str) -> Result<String, Box<dyn Error>> {
    env::var(key)
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, format!("{key} is required")).into())
}

fn read_private_key() -> Result<String, Box<dyn Error>> {
    if let Ok(pem) = env::var("APPLE_PRIVATE_KEY_PEM") {
        if !pem.trim().is_empty() {
            return Ok(pem);
        }
    }

    let path = required_env("APPLE_PRIVATE_KEY_PATH")?;
    Ok(std::fs::read_to_string(path)?)
}

fn parse_ttl() -> Result<u64, Box<dyn Error>> {
    let seconds = env::var("SOSUMI_TOKEN_TTL_SECONDS")
        .ok()
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(3_600);

    if seconds == 0 || Duration::from_secs(seconds) > MAX_TOKEN_TTL {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "SOSUMI_TOKEN_TTL_SECONDS must be between 1 and {}",
                MAX_TOKEN_TTL.as_secs()
            ),
        )
        .into());
    }

    Ok(seconds)
}

fn bind_address() -> Result<SocketAddr, Box<dyn Error>> {
    let bind = env::var("SOSUMI_TOKEN_BIND").unwrap_or_else(|_| "127.0.0.1:8787".to_owned());
    Ok(bind.parse()?)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        warn!(%error, "failed to install shutdown signal handler");
    }
}
