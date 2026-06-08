use disyn_core::types::{PlanDraft, VerificationLayer, Violation};

pub trait LayerRule: Send + Sync {
    fn layer(&self) -> VerificationLayer;
    fn rule_id(&self) -> &str;
    fn check(&self, draft: &PlanDraft) -> Vec<Violation>;
}
