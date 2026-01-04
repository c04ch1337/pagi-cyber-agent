use serde::{Deserialize, Serialize};
use sled::Db;

/// A simplified snapshot of enterprise security tooling state.
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub zscaler_status: String,
    pub crowdstrike_endpoint_count: u32,
    pub proofpoint_quarantined_emails: u32,
    pub jira_open_tickets: u32,
    pub meraki_network_health: String,
}

const SECURITY_POLICY_TREE: &str = "security_policy_tree";
const SECURITY_POLICY_KEY: &[u8] = b"current";

fn default_policy() -> SecurityPolicy {
    // NOTE: intentionally sets a low CrowdStrike endpoint count to exercise the
    // "symbolic rule creation" branch in the agent.
    SecurityPolicy {
        zscaler_status: "OK".to_string(),
        crowdstrike_endpoint_count: 42,
        proofpoint_quarantined_emails: 3,
        jira_open_tickets: 7,
        meraki_network_health: "DEGRADED".to_string(),
    }
}

/// Simulates loading the current security policy from a dedicated Sled tree.
///
/// This is intentionally a "best effort" loader. If the policy isn't present yet,
/// it seeds a default policy into the tree and returns it.
pub fn load_policy(db: &Db) -> SecurityPolicy {
    let Ok(tree) = db.open_tree(SECURITY_POLICY_TREE) else {
        return default_policy();
    };

    if let Ok(Some(raw)) = tree.get(SECURITY_POLICY_KEY) {
        if let Ok(policy) = serde_json::from_slice::<SecurityPolicy>(&raw) {
            return policy;
        }
    }

    let policy = default_policy();
    if let Ok(encoded) = serde_json::to_vec(&policy) {
        let _ = tree.insert(SECURITY_POLICY_KEY, encoded);
        let _ = tree.flush();
    }
    policy
}
