# SpendGuard — BTL Runtime Cost Governance Layer

> Point any BTL Runtime app at SpendGuard instead. Zero code changes.  
> Per-agent budgets. Live savings telemetry. Kill-switch in one click.

---

## The Problem

Every team building on BTL Runtime saves money through caching — but only  
finds out how much at month-end. There is no per-agent budget enforcement,  
no live visibility into which agents are burning budget, and no way to stop  
a runaway workflow before the invoice arrives.

Fintech agents (KYC bots, tax assistants, compliance pipelines) run thousands  
of calls per day. A single misconfigured prompt loop can erase a month's  
budget in hours. Today there is no guardrail.

---

## What SpendGuard Does

SpendGuard is an OpenAI-compatible proxy that sits between your application  
and BTL Runtime. It:

1. **Enforces per-agent monthly budgets** — blocks requests before they exceed limits
2. **Surfaces BTL's own telemetry** — reads `x-btl-cache-tier`, `x-btl-benchmark-cost`,  
   `x-btl-customer-charge`, `x-btl-saved` and makes them visible in real time
3. **Streams live telemetry** — every call appears on the dashboard within milliseconds  
   via WebSocket broadcast
4. **Kill-switch** — any agent can be stopped instantly from the UI, with immediate  
   enforcement on the next proxied call
5. **Audit ledger** — every request is logged with full BTL cost metadata for  
   post-hoc analysis

---

## BTL Runtime Integration

SpendGuard's value proposition is **100% derived from BTL Runtime's own headers**.  
We do not estimate or re-derive cost data client-side.

| BTL Header | How SpendGuard Uses It |
|---|---|
| `x-btl-cache-tier` | Labels every request EXACT / SEMANTIC / MISS in the live feed |
| `x-btl-benchmark-cost` | Shown as "what you would have paid" |
| `x-btl-customer-charge` | Debited against the agent's monthly budget |
| `x-btl-saved` | Powers the cumulative savings counter and chart |

**To redirect any existing BTL Runtime app to SpendGuard:**
```bash
# Before
OPENAI_BASE_URL=https://runtime.badtheorylabs.com/v1

# After
OPENAI_BASE_URL=http://your-spendguard-host/v1/proxy?agent_id=<id>
```
No other code changes required.

---

## Demo

The dashboard runs in  a fully live backend and frontend 
The frontend link is https://spend-guard-frontend-f7rw.vercel.app/
The railway backend link is https://spendguard-backend-production-0407.up.railway.app/v1/proxy/chat/completions?agent_id=a3df3872-d1d7-4e06-a39b-a3a42989bf7e

---

## Technical Stack

**Backend:** Rust / Axum — DDD workspace (domain / infra / api layers)  
**Database:** SQLite via SQLx — schema migrations on boot, index on `(agent_id, ts)`  
**Realtime:** Tokio broadcast channel → WebSocket — telemetry never blocks the proxy path  
**Frontend:** Vite / React / Recharts — JetBrains Mono, obsidian/cyan palette  
**Safety:** `VITE_MOCK_MODE` flag — demo is fully functional with no live dependencies  
**Money:** All cost values stored and computed in integer cents — no floating point

---

## Why This Becomes a Company

Every team building production AI agents on any LLM gateway has this problem.  
SpendGuard is gateway-agnostic at the product level — the BTL header names are  
the only integration point. The audit ledger accumulates proprietary per-customer  
cost-efficiency data that compounds over time into benchmarking intelligence  
no competitor can replicate without the same install base.

The natural expansion: anomaly detection → cost forecasting → cross-gateway  
arbitrage recommendations → managed routing.

Seed ask: $500K. Use of funds: multi-gateway support, team billing, SOC 2.

---

## Running Locally

```bash
# Backend
cd backend
cp .env.example .env        # add BTL_API_KEY
cargo run

# Frontend (demo mode — no backend needed)
cd frontend
npm install
npm run dev                  # VITE_MOCK_MODE=true
```

---

## Repository Structure
spendguard/
backend/
src/
domain/         # Agent, Workflow, RequestLog, Budget — pure Rust, no I/O
infra/          # SQLite (db.rs) + BTL Runtime client (btl_client.rs)
api/            # proxy.rs, agents.rs, telemetry_ws.rs
app_state.rs    # shared Db + BtlClient + broadcast channel
main.rs         # Axum router, CORS, server boot
frontend/
src/
lib/mock.ts           # VITE_MOCK_MODE simulation
components/
AgentList.tsx        # sidebar with circular budget gauges
BudgetGauge.tsx      # SVG arc gauge, colour-coded by status
SavingsLedger.tsx    # top stat bar — savings, hit rate, spend, requests
LiveTelemetryFeed.tsx # scrolling real-time request log
SpendChart.tsx       # cumulative savings vs spend area chart
CreateAgentModal.tsx # provision new agent from UI
App.tsx               # shell, WebSocket connection, mock simulation loop
---

*Built for the runtime. hackathon — Bad Theory Labs, July 2026*  
*Branham47 Labs — Kisumu, Kenya*
