use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub monthly_budget_cents: i64,
    pub current_spend_cents: i64,
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,
    Throttled,
    Killed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetDecision {
    Allow,
    AllowWithWarning,
    Block,
}

const WARNING_THRESHOLD_PCT: f64 = 80.0;

impl Agent {
    pub fn new(name: String, monthly_budget_cents: i64) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            monthly_budget_cents,
            current_spend_cents: 0,
            status: AgentStatus::Active,
        }
    }

    pub fn spend_pct(&self) -> f64 {
        if self.monthly_budget_cents <= 0 {
            return 0.0;
        }
        (self.current_spend_cents as f64 / self.monthly_budget_cents as f64) * 100.0
    }

    pub fn evaluate_charge(&self, prospective_charge_cents: i64) -> BudgetDecision {
        if self.status == AgentStatus::Killed {
            return BudgetDecision::Block;
        }

        let projected = self.current_spend_cents + prospective_charge_cents;
        if projected > self.monthly_budget_cents {
            return BudgetDecision::Block;
        }

        let projected_pct = if self.monthly_budget_cents > 0 {
            (projected as f64 / self.monthly_budget_cents as f64) * 100.0
        } else {
            0.0
        };

        if projected_pct >= WARNING_THRESHOLD_PCT {
            BudgetDecision::AllowWithWarning
        } else {
            BudgetDecision::Allow
        }
    }

    pub fn record_charge(&mut self, charge_cents: i64) {
        self.current_spend_cents += charge_cents;
        if self.spend_pct() >= 100.0 {
            self.status = AgentStatus::Throttled;
        }
    }

    pub fn kill(&mut self) {
        self.status = AgentStatus::Killed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_charge_within_budget() {
        let agent = Agent::new("kyc-bot".into(), 10_000);
        assert_eq!(agent.evaluate_charge(1_000), BudgetDecision::Allow);
    }

    #[test]
    fn warns_at_threshold() {
        let mut agent = Agent::new("kyc-bot".into(), 10_000);
        agent.record_charge(7_500);
        assert_eq!(agent.evaluate_charge(500), BudgetDecision::AllowWithWarning);
    }

    #[test]
    fn blocks_over_budget() {
        let mut agent = Agent::new("kyc-bot".into(), 10_000);
        agent.record_charge(9_500);
        assert_eq!(agent.evaluate_charge(1_000), BudgetDecision::Block);
    }

    #[test]
    fn killed_agent_always_blocked() {
        let mut agent = Agent::new("kyc-bot".into(), 10_000);
        agent.kill();
        assert_eq!(agent.evaluate_charge(1), BudgetDecision::Block);
    }
}