use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderValue, Method, StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use reqwest::Client;
use sqlx::Executor;
use tower_http::{
    LatencyUnit,
    cors::{AllowOrigin, CorsLayer},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

use crate::{
    config::{db::DbPool, environment::Environment},
    module::admin::route::router as admin_router,
    module::auth::route::router as auth_router,
    module::compliance::route::{
        admin_router as admin_compliance_router, public_router as public_compliance_router,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub env: Environment,
    pub http_client: Client,
}

pub fn build_router(state: AppState) -> Result<Router> {
    let cors_layer = build_cors_layer(&state.env)?;

    Ok(Router::new()
        .route("/health", get(health_check))
        .nest(
            "/admin",
            admin_router(state.clone()).merge(admin_compliance_router(state.clone())),
        )
        .nest("/auth", auth_router(state.clone()))
        .merge(public_compliance_router())
        .with_state(state)
        .layer(cors_layer)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Millis),
                ),
        ))
}

fn build_cors_layer(env: &Environment) -> Result<CorsLayer> {
    let allowed_origins = env
        .cors_allowed_origins
        .iter()
        .map(|origin| {
            HeaderValue::from_str(origin).with_context(|| {
                format!("invalid CORS_ALLOWED_ORIGINS/CORS_ALLOWED_ORIGIN value `{origin}`")
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::ACCEPT, header::AUTHORIZATION, header::CONTENT_TYPE]))
}

#[derive(serde::Serialize)]
struct HealthResponse<'a> {
    status: &'a str,
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.execute("SELECT 1").await {
        Ok(_) => (StatusCode::OK, Json(HealthResponse { status: "ok" })),
        Err(error) => {
            tracing::error!(?error, "database health check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthResponse { status: "degraded" }),
            )
        }
    }
}
