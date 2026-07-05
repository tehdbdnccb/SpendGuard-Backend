use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::{self, Agent, Workflow};

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub monthly_budget_cents: i64,
}

pub async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> impl IntoResponse {
    let agent = Agent::new(req.name, req.monthly_budget_cents);
    match state.db.insert_agent(&agent).await {
        Ok(_) => (StatusCode::CREATED, Json(agent)).into_response(),
        Err(e) => { tracing::error!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to create agent" }))).into_response() }
    }
}

pub async fn list_agents(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.list_agents().await {
        Ok(agents) => (StatusCode::OK, Json(agents)).into_response(),
        Err(e) => { tracing::error!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to list agents" }))).into_response() }
    }
}

pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.db.get_agent(id).await {
        Ok(Some(agent)) => (StatusCode::OK, Json(agent)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "agent not found" }))).into_response(),
        Err(e) => { tracing::error!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal_error" }))).into_response() }
    }
}

pub async fn get_agent_ledger(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let logs = match state.db.list_logs_for_agent(id, 500).await {
        Ok(l) => l,
        Err(e) => { tracing::error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal_error" }))).into_response(); }
    };
    let summary = domain::summarize(&logs);
    (StatusCode::OK, Json(serde_json::json!({ "summary": summary, "recent_logs": logs }))).into_response()
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkflowRequest {
    pub agent_id: Uuid,
    pub name: String,
    pub kill_switch_threshold_pct: f64,
}

pub async fn create_workflow(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateWorkflowRequest>,
) -> impl IntoResponse {
    let workflow = Workflow::new(req.agent_id, req.name, req.kill_switch_threshold_pct);
    match state.db.insert_workflow(&workflow).await {
        Ok(_) => (StatusCode::CREATED, Json(workflow)).into_response(),
        Err(e) => { tracing::error!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to create workflow" }))).into_response() }
    }
}

pub async fn kill_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let mut agent = match state.db.get_agent(id).await {
        Ok(Some(a)) => a,
        Ok(None) => return (StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "agent not found" }))).into_response(),
        Err(e) => { tracing::error!("{e}");
            return (StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "internal_error" }))).into_response(); }
    };
    agent.kill();
    match state.db.update_agent(&agent).await {
        Ok(_) => (StatusCode::OK, Json(agent)).into_response(),
        Err(e) => { tracing::error!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "failed to kill agent" }))).into_response() }
    }
}