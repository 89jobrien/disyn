use disyn_core::types::{PlanDraft, VerificationLayer, Violation};

// TODO: Add unit tests for each LayerRule implementation — the trait contract (layer, rule_id,
// check) should be exercised with both passing and violating PlanDraft fixtures.
pub trait LayerRule: Send + Sync {
    fn layer(&self) -> VerificationLayer;
    fn rule_id(&self) -> &str;
    fn check(&self, draft: &PlanDraft) -> Vec<Violation>;
}
