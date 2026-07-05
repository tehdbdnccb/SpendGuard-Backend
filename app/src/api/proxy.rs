use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::domain::{self, AgentStatus, BudgetDecision, RequestLog};

#[derive(Debug, Deserialize)]
pub struct ProxyScope {
    pub agent_id: Uuid,
    pub workflow_id: Option<Uuid>,
}

const PREFLIGHT_ESTIMATE_CENTS: i64 = 5;

pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Query(scope): Query<ProxyScope>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let agent = match state.db.get_agent(scope.agent_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return (StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "unknown agent_id" }))).into_response(),
        Err(e) => return internal_error(e),
    };

    let workflow = match scope.workflow_id {
        Some(wf_id) => match state.db.get_workflow(wf_id).await {
            Ok(wf) => wf,
            Err(e) => return internal_error(e),
        },
        None => None,
    };

    let verdict = domain::evaluate(&agent, workflow.as_ref(), PREFLIGHT_ESTIMATE_CENTS);

    if verdict.decision == BudgetDecision::Block {
        return (StatusCode::PAYMENT_REQUIRED, Json(serde_json::json!({
            "error": "budget_blocked",
            "reason": verdict.reason,
            "agent_spend_pct": verdict.agent_spend_pct,
        }))).into_response();
    }

    let btl_response = match state.btl.chat_completion(body).await {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": format!("BTL Runtime error: {e}") }))).into_response(),
    };

    let model = btl_response.body.get("model")
        .and_then(|v| v.as_str()).unwrap_or("unknown").to_string();

    let log = RequestLog::new(
        agent.id,
        scope.workflow_id,
        model,
        btl_response.telemetry.cache_tier,
        btl_response.telemetry.benchmark_cost_cents,
        btl_response.telemetry.customer_charge_cents,
        btl_response.telemetry.saved_cents,
        btl_response.telemetry.latency_ms,
    );

    if let Err(e) = state.db.insert_request_log(&log).await {
        tracing::error!("failed to persist request log: {e}");
    }

    let mut updated_agent = agent;
    updated_agent.record_charge(btl_response.telemetry.customer_charge_cents);
    if let Err(e) = state.db.update_agent(&updated_agent).await {
        tracing::error!("failed to update agent spend: {e}");
    }

    if updated_agent.status == AgentStatus::Throttled {
        tracing::warn!("agent '{}' crossed monthly budget", updated_agent.name);
    }

    state.publish_telemetry(log);

    (StatusCode::OK, Json(btl_response.body)).into_response()
}

fn internal_error(e: anyhow::Error) -> axum::response::Response {
    tracing::error!("internal error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": "internal_error" }))).into_response()
}