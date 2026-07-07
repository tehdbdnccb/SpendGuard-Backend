mod api;
mod app_state;
mod domain;
mod infra;
mod middleware;

use std::sync::Arc;
use axum::{routing::{get, post}, Router};
use axum::middleware as axum_middleware;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use app_state::AppState;
use infra::{BtlClient, Db};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("spendguard=debug".parse()?),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/spendguard".to_string());
    let db = Db::connect(&database_url).await?;
    let btl = BtlClient::from_env()?;
    let state = Arc::new(AppState::new(db, btl));

    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let protected = Router::new()
    .route("/agents/:id", get(api::agents::get_agent))
    .route("/agents/:id/ledger", get(api::agents::get_agent_ledger))
    .route("/agents/:id/kill-switch", post(api::agents::kill_agent))
    .route("/agents", post(api::agents::create_agent))
    .route("/workflows", post(api::agents::create_workflow))
    .route("/admin/agents/:id/set-spend", post(api::agents::set_agent_spend))
    .layer(axum_middleware::from_fn(middleware::api_key_auth));

let app = Router::new()
    .route("/v1/proxy/chat/completions", post(api::proxy::chat_completions))
    .route("/ws/telemetry", get(api::telemetry_ws::telemetry_ws))
    .route("/agents", get(api::agents::list_agents))
    .merge(protected)
    .layer(cors)
    .layer(TraceLayer::new_for_http())
    .with_state(state);
    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3001".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("SpendGuard listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

