//! Contract for layer-specific symbolic plan rules.

use disyn_core::types::{PlanDraft, VerificationLayer, Violation};

// TODO: Add unit tests for each LayerRule implementation — the trait contract (layer, rule_id,
// check) should be exercised with both passing and violating PlanDraft fixtures.
pub trait LayerRule: Send + Sync {
    /// Returns the verification layer that orders this rule.
    fn layer(&self) -> VerificationLayer;
    /// Returns the stable identifier included in reported violations.
    fn rule_id(&self) -> &str;
    /// Checks a draft and returns every violation detected by this rule.
    fn check(&self, draft: &PlanDraft) -> Vec<Violation>;
}
