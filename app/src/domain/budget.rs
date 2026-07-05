use serde::Serialize;
use super::agent::{Agent, AgentStatus, BudgetDecision};
use super::workflow::Workflow;

#[derive(Debug, Clone, Serialize)]
pub struct BudgetVerdict {
    pub decision: BudgetDecision,
    pub reason: String,
    pub agent_spend_pct: f64,
}

pub fn evaluate(
    agent: &Agent,
    workflow: Option<&Workflow>,
    prospective_charge_cents: i64,
) -> BudgetVerdict {
    let projected_spend_pct = if agent.monthly_budget_cents > 0 {
        ((agent.current_spend_cents + prospective_charge_cents) as f64
            / agent.monthly_budget_cents as f64)
            * 100.0
    } else {
        0.0
    };

    if let Some(wf) = workflow {
        if wf.should_trip(projected_spend_pct) {
            return BudgetVerdict {
                decision: BudgetDecision::Block,
                reason: format!(
                    "workflow '{}' kill-switch threshold ({:.0}%) breached at {:.1}% agent spend",
                    wf.name, wf.kill_switch_threshold_pct, projected_spend_pct
                ),
                agent_spend_pct: projected_spend_pct,
            };
        }
    }

    let decision = agent.evaluate_charge(prospective_charge_cents);
    let reason = match decision {
        BudgetDecision::Allow => "within budget".to_string(),
        BudgetDecision::AllowWithWarning => format!(
            "agent '{}' at {:.1}% of monthly budget",
            agent.name, projected_spend_pct
        ),
        BudgetDecision::Block => match agent.status {
            AgentStatus::Killed => format!("agent '{}' is killed", agent.name),
            _ => format!(
                "agent '{}' would exceed monthly budget ({:.1}%)",
                agent.name, projected_spend_pct
            ),
        },
    };

    BudgetVerdict {
        decision,
        reason,
        agent_spend_pct: projected_spend_pct,
    }
}
