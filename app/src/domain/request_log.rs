use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub workflow_id: Option<Uuid>,
    pub model: String,
    pub btl_cache_tier: CacheTier,
    pub benchmark_cost_cents: i64,
    pub customer_charge_cents: i64,
    pub saved_cents: i64,
    pub latency_ms: u32,
    pub ts: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheTier {
    ExactResponseCache,
    SemanticCache,
    NoCache,
}

impl CacheTier {
    pub fn from_header(raw: &str) -> Self {
        match raw {
            "exact_response_cache" => CacheTier::ExactResponseCache,
            "semantic_cache" => CacheTier::SemanticCache,
            _ => CacheTier::NoCache,
        }
    }

    pub fn is_cache_hit(&self) -> bool {
        !matches!(self, CacheTier::NoCache)
    }
}

impl RequestLog {
    pub fn new(
        agent_id: Uuid,
        workflow_id: Option<Uuid>,
        model: String,
        btl_cache_tier: CacheTier,
        benchmark_cost_cents: i64,
        customer_charge_cents: i64,
        saved_cents: i64,
        latency_ms: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            workflow_id,
            model,
            btl_cache_tier,
            benchmark_cost_cents,
            customer_charge_cents,
            saved_cents,
            latency_ms,
            ts: Utc::now(),
        }
    }
}

pub fn summarize(logs: &[RequestLog]) -> LedgerSummary {
    let total_requests = logs.len() as u64;
    let cache_hits = logs.iter().filter(|l| l.btl_cache_tier.is_cache_hit()).count() as u64;
    let total_saved_cents: i64 = logs.iter().map(|l| l.saved_cents).sum();
    let total_charged_cents: i64 = logs.iter().map(|l| l.customer_charge_cents).sum();

    let cache_hit_rate_pct = if total_requests > 0 {
        (cache_hits as f64 / total_requests as f64) * 100.0
    } else {
        0.0
    };

    LedgerSummary {
        total_requests,
        cache_hits,
        cache_hit_rate_pct,
        total_saved_cents,
        total_charged_cents,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LedgerSummary {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_hit_rate_pct: f64,
    pub total_saved_cents: i64,
    pub total_charged_cents: i64,
}