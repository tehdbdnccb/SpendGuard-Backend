use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub name: String,
    pub kill_switch_threshold_pct: f64,
    pub enabled: bool,
}

impl Workflow {
    pub fn new(agent_id: Uuid, name: String, kill_switch_threshold_pct: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id,
            name,
            kill_switch_threshold_pct,
            enabled: true,
        }
    }

    pub fn should_trip(&self, agent_spend_pct: f64) -> bool {
        self.enabled && agent_spend_pct >= self.kill_switch_threshold_pct
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trips_when_threshold_breached() {
        let wf = Workflow::new(Uuid::new_v4(), "document-verification".into(), 70.0);
        assert!(wf.should_trip(75.0));
        assert!(!wf.should_trip(50.0));
    }

    #[test]
    fn disabled_workflow_never_trips() {
        let mut wf = Workflow::new(Uuid::new_v4(), "address-match".into(), 50.0);
        wf.disable();
        assert!(!wf.should_trip(99.0));
    }
}