mod cybersecurity_agent;
mod policy_manager;

use cybersecurity_agent::CybersecurityAgent;
use pagi_core_lib::BaseAgent;

#[tokio::main]
async fn main() {
    // Test: simulate a triage run.
    let agent = CybersecurityAgent::new();
    let result = agent
        .run("HIGH_SEVERITY_ALERT: Source=Rapid7 SIEM, User=Alice")
        .await;
    println!("{result}");
}

