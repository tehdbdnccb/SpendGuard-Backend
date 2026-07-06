use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

use crate::domain::{Agent, AgentStatus, CacheTier, RequestLog, Workflow};

#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                monthly_budget_cents INTEGER NOT NULL,
                current_spend_cents INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'active'
            );"
        ).execute(&self.pool).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL REFERENCES agents(id),
                name TEXT NOT NULL,
                kill_switch_threshold_pct REAL NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1
            );"
        ).execute(&self.pool).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS request_logs (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL REFERENCES agents(id),
                workflow_id TEXT REFERENCES workflows(id),
                model TEXT NOT NULL,
                btl_cache_tier TEXT NOT NULL,
                benchmark_cost_cents INTEGER NOT NULL,
                customer_charge_cents INTEGER NOT NULL,
                saved_cents INTEGER NOT NULL,
                latency_ms INTEGER NOT NULL,
                ts TEXT NOT NULL
            );"
        ).execute(&self.pool).await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_request_logs_agent_ts ON request_logs(agent_id, ts);"
        ).execute(&self.pool).await?;

        Ok(())
    }

    pub async fn insert_agent(&self, agent: &Agent) -> Result<()> {
        sqlx::query(
            "INSERT INTO agents (id, name, monthly_budget_cents, current_spend_cents, status)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(agent.id.to_string())
        .bind(&agent.name)
        .bind(agent.monthly_budget_cents)
        .bind(agent.current_spend_cents)
        .bind(status_to_str(agent.status))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_agent(&self, id: Uuid) -> Result<Option<Agent>> {
        let row = sqlx::query_as::<_, AgentRow>("SELECT * FROM agents WHERE id = $1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Into::into))
    }

    pub async fn list_agents(&self) -> Result<Vec<Agent>> {
        let rows = sqlx::query_as::<_, AgentRow>("SELECT * FROM agents")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn update_agent(&self, agent: &Agent) -> Result<()> {
        sqlx::query(
            "UPDATE agents SET current_spend_cents = $1, status = $2 WHERE id = $3",
        )
        .bind(agent.current_spend_cents)
        .bind(status_to_str(agent.status))
        .bind(agent.id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_workflow(&self, wf: &Workflow) -> Result<()> {
        sqlx::query(
            "INSERT INTO workflows (id, agent_id, name, kill_switch_threshold_pct, enabled)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(wf.id.to_string())
        .bind(wf.agent_id.to_string())
        .bind(&wf.name)
        .bind(wf.kill_switch_threshold_pct)
        .bind(wf.enabled as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_workflow(&self, id: Uuid) -> Result<Option<Workflow>> {
        let row = sqlx::query_as::<_, WorkflowRow>("SELECT * FROM workflows WHERE id = $1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Into::into))
    }

    pub async fn insert_request_log(&self, log: &RequestLog) -> Result<()> {
        sqlx::query(
            "INSERT INTO request_logs
             (id, agent_id, workflow_id, model, btl_cache_tier, benchmark_cost_cents,
              customer_charge_cents, saved_cents, latency_ms, ts)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(log.id.to_string())
        .bind(log.agent_id.to_string())
        .bind(log.workflow_id.map(|w| w.to_string()))
        .bind(&log.model)
        .bind(cache_tier_to_str(log.btl_cache_tier))
        .bind(log.benchmark_cost_cents)
        .bind(log.customer_charge_cents)
        .bind(log.saved_cents)
        .bind(log.latency_ms as i64)
        .bind(log.ts.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_logs_for_agent(&self, agent_id: Uuid, limit: i64) -> Result<Vec<RequestLog>> {
        let rows = sqlx::query_as::<_, RequestLogRow>(
            "SELECT * FROM request_logs WHERE agent_id = $1 ORDER BY ts DESC LIMIT $2",
        )
        .bind(agent_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[derive(sqlx::FromRow)]
struct AgentRow {
    id: String,
    name: String,
    monthly_budget_cents: i64,
    current_spend_cents: i64,
    status: String,
}

impl From<AgentRow> for Agent {
    fn from(r: AgentRow) -> Self {
        Agent {
            id: Uuid::parse_str(&r.id).unwrap_or_default(),
            name: r.name,
            monthly_budget_cents: r.monthly_budget_cents,
            current_spend_cents: r.current_spend_cents,
            status: status_from_str(&r.status),
        }
    }
}

#[derive(sqlx::FromRow)]
struct WorkflowRow {
    id: String,
    agent_id: String,
    name: String,
    kill_switch_threshold_pct: f64,
    enabled: i64,
}

impl From<WorkflowRow> for Workflow {
    fn from(r: WorkflowRow) -> Self {
        Workflow {
            id: Uuid::parse_str(&r.id).unwrap_or_default(),
            agent_id: Uuid::parse_str(&r.agent_id).unwrap_or_default(),
            name: r.name,
            kill_switch_threshold_pct: r.kill_switch_threshold_pct,
            enabled: r.enabled != 0,
        }
    }
}

#[derive(sqlx::FromRow)]
struct RequestLogRow {
    id: String,
    agent_id: String,
    workflow_id: Option<String>,
    model: String,
    btl_cache_tier: String,
    benchmark_cost_cents: i64,
    customer_charge_cents: i64,
    saved_cents: i64,
    latency_ms: i64,
    ts: String,
}

impl From<RequestLogRow> for RequestLog {
    fn from(r: RequestLogRow) -> Self {
        RequestLog {
            id: Uuid::parse_str(&r.id).unwrap_or_default(),
            agent_id: Uuid::parse_str(&r.agent_id).unwrap_or_default(),
            workflow_id: r.workflow_id.and_then(|s| Uuid::parse_str(&s).ok()),
            model: r.model,
            btl_cache_tier: cache_tier_from_str(&r.btl_cache_tier),
            benchmark_cost_cents: r.benchmark_cost_cents,
            customer_charge_cents: r.customer_charge_cents,
            saved_cents: r.saved_cents,
            latency_ms: r.latency_ms as u32,
            ts: chrono::DateTime::parse_from_rfc3339(&r.ts)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
        }
    }
}

fn status_to_str(s: AgentStatus) -> &'static str {
    match s {
        AgentStatus::Active => "active",
        AgentStatus::Throttled => "throttled",
        AgentStatus::Killed => "killed",
    }
}

fn status_from_str(s: &str) -> AgentStatus {
    match s {
        "throttled" => AgentStatus::Throttled,
        "killed" => AgentStatus::Killed,
        _ => AgentStatus::Active,
    }
}

fn cache_tier_to_str(t: CacheTier) -> &'static str {
    match t {
        CacheTier::ExactResponseCache => "exact_response_cache",
        CacheTier::SemanticCache => "semantic_cache",
        CacheTier::NoCache => "no_cache",
    }
}

fn cache_tier_from_str(s: &str) -> CacheTier {
    match s {
        "exact_response_cache" => CacheTier::ExactResponseCache,
        "semantic_cache" => CacheTier::SemanticCache,
        _ => CacheTier::NoCache,
    }
}

