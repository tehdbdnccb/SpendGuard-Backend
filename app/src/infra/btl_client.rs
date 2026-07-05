use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::time::Instant;

use crate::domain::CacheTier;

#[derive(Clone)]
pub struct BtlClient {
    http: Client,
    base_url: String,
    api_key: String,
}

#[derive(Debug, Clone)]
pub struct BtlTelemetry {
    pub cache_tier: CacheTier,
    pub benchmark_cost_cents: i64,
    pub customer_charge_cents: i64,
    pub saved_cents: i64,
    pub latency_ms: u32,
}

pub struct BtlResponse {
    pub body: Value,
    pub telemetry: BtlTelemetry,
}

impl BtlClient {
    pub fn from_env() -> Result<Self> {
        let base_url = env::var("BTL_RUNTIME_BASE_URL")
            .unwrap_or_else(|_| "https://api.badtheorylabs.com/v1".to_string());
        let api_key = env::var("BTL_API_KEY")
            .map_err(|_| anyhow!("BTL_API_KEY not set in environment"))?;

        Ok(Self {
            http: Client::new(),
            base_url,
            api_key,
        })
    }

    pub async fn chat_completion(&self, body: Value) -> Result<BtlResponse> {
        let started = Instant::now();

        let resp = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let latency_ms = started.elapsed().as_millis() as u32;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("BTL Runtime returned {status}: {text}"));
        }

        let cache_tier = header_str(&resp, "x-btl-cache-tier")
            .map(|s| CacheTier::from_header(&s))
            .unwrap_or(CacheTier::NoCache);

        let benchmark_cost_cents = header_dollars_to_cents(&resp, "x-btl-benchmark-cost");
        let customer_charge_cents = header_dollars_to_cents(&resp, "x-btl-customer-charge");
        let saved_cents = header_dollars_to_cents(&resp, "x-btl-saved");

        let telemetry = BtlTelemetry {
            cache_tier,
            benchmark_cost_cents,
            customer_charge_cents,
            saved_cents,
            latency_ms,
        };

        let body: Value = resp.json().await?;
        Ok(BtlResponse { body, telemetry })
    }
}

fn header_str(resp: &reqwest::Response, name: &str) -> Option<String> {
    resp.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn header_dollars_to_cents(resp: &reqwest::Response, name: &str) -> i64 {
    header_str(resp, name)
        .and_then(|s| s.parse::<f64>().ok())
        .map(|dollars| (dollars * 100.0).round() as i64)
        .unwrap_or(0)
}