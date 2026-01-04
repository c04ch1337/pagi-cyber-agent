use async_trait::async_trait;
use pagi_core_lib::{BaseAgent, Fact, PAGICoreModel, DEFAULT_KNOWLEDGE_BASE_PATH};
use serde::{Deserialize, Serialize};
use sled::Db;

use crate::policy_manager;

const RULES_TREE: &str = "rules";

/// A minimal, symbolic rule record persisted by this agent.
///
/// Notes:
/// - This is intentionally local to `pagi-cyber-agent` (not provided by `pagi-core-lib`).
/// - Rules are persisted into the KB tree `rules` as JSON blobs for later consumption by an
///   orchestrator / rule engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PAGIRule {
    pub id: String,
    pub condition_fact_type: String,
    pub condition_keyword: String,
    pub action_directive: String,
}

pub struct CybersecurityAgent {
    agent_id: String,
    db: Db,
    core: PAGICoreModel,
}

impl CybersecurityAgent {
    pub fn new() -> Self {
        let db = sled::open(DEFAULT_KNOWLEDGE_BASE_PATH)
            .expect("failed to open sled knowledge base for CybersecurityAgent");
        let core = PAGICoreModel::from_db(db.clone());
        Self {
            agent_id: "CybersecurityAgent".to_string(),
            db,
            core,
        }
    }

    fn now_unix_seconds() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn write_rule_to_kb(&self, rule: &PAGIRule) {
        let Ok(tree) = self.db.open_tree(RULES_TREE) else {
            return;
        };

        let Ok(value) = serde_json::to_vec(rule) else {
            return;
        };

        let _ = tree.insert(rule.id.as_bytes(), value);
        let _ = tree.flush();
    }
}

#[async_trait]
impl BaseAgent for CybersecurityAgent {
    async fn run(&self, task_input: &str) -> String {
        // Initial action: load current security state/policy.
        let policy = policy_manager::load_policy(&self.db);

        // Triage simulation.
        let mut plan_directive = "ORCHESTRATE_RESPONSE: monitor".to_string();
        if task_input.contains("HIGH_SEVERITY_ALERT") {
            plan_directive =
                "ORCHESTRATE_RESPONSE: block_user, investigate_logs, create_ticket".to_string();
        }

        // Symbolic rule creation: if we detect a severe imbalance, write a new rule into KB.
        let mut rule_written: Option<PAGIRule> = None;
        if policy.crowdstrike_endpoint_count < 100 {
            let id = self
                .db
                .generate_id()
                .map(|n| format!("rule_crowdstrike_{n}"))
                .unwrap_or_else(|_| "rule_crowdstrike_fallback".to_string());

            let rule = PAGIRule {
                id,
                condition_fact_type: "SecurityTriage".to_string(),
                condition_keyword: "Crowdstrike".to_string(),
                action_directive: "Send Alert to Jira".to_string(),
            };

            self.write_rule_to_kb(&rule);
            rule_written = Some(rule);
        }

        // Write fact to KB.
        let mut content = serde_json::json!({
            "task_input": task_input,
            "plan_directive": plan_directive,
            "policy_snapshot": {
                "zscaler_status": policy.zscaler_status,
                "crowdstrike_endpoint_count": policy.crowdstrike_endpoint_count,
                "proofpoint_quarantined_emails": policy.proofpoint_quarantined_emails,
                "jira_open_tickets": policy.jira_open_tickets,
                "meraki_network_health": policy.meraki_network_health,
            }
        });

        if let Some(rule) = &rule_written {
            content["rule_written"] = serde_json::to_value(rule).unwrap_or(serde_json::Value::Null);
        }

        let fact = Fact {
            agent_id: self.agent_id.clone(),
            timestamp: Self::now_unix_seconds(),
            fact_type: "SecurityTriage".to_string(),
            content: content.to_string(),
        };

        let _ = self.core.record_fact(fact).await;

        format!(
            "Cybersecurity triage complete. directive={}; rule_written={}",
            plan_directive,
            rule_written
                .as_ref()
                .map(|r| r.id.as_str())
                .unwrap_or("none")
        )
    }
}
