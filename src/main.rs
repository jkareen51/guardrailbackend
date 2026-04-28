use anyhow::Result;
use guardrailbackend::{
    app::{AppState, build_router},
    config::{db::create_pool, environment::Environment},
};
use reqwest::Client;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let env = Environment::load()?;
    let db = create_pool(&env).await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    let address = env.bind_address();
    let state = AppState {
        db,
        env,
        http_client: Client::new(),
    };
    let app = build_router(state)?;
    let listener = TcpListener::bind(address).await?;

    tracing::info!(%address, "server listening");
    axum::serve(listener, app).await?;

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("guardrailbackend=debug,tower_http=info")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
