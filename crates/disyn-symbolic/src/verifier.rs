use disyn_core::ports::Verifier;
use disyn_core::types::{PlanDraft, Severity, VerificationReport, Violation};

#[derive(Default)]
pub struct RuleSetVerifier;

impl Verifier for RuleSetVerifier {
    fn verify(&self, draft: &PlanDraft) -> VerificationReport {
        let mut violations = Vec::new();
        for (i, step) in draft.steps.iter().enumerate() {
            if step.action.is_empty() {
                violations.push(Violation {
                    rule_id: "non-empty-action".into(),
                    severity: Severity::Blocking,
                    message: "step action must not be empty".into(),
                    step_index: i,
                });
            }
        }
        VerificationReport {
            passed: violations.is_empty(),
            violations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use disyn_core::types::{CostEstimate, PlannedStep};

    #[test]
    fn empty_plan_passes() {
        let v = RuleSetVerifier;
        let draft = PlanDraft {
            steps: vec![],
            rationale: "no-op".into(),
        };
        let report = v.verify(&draft);
        assert!(report.passed);
    }

    #[test]
    fn step_with_empty_action_fails() {
        let v = RuleSetVerifier;
        let draft = PlanDraft {
            steps: vec![PlannedStep {
                action: "".into(),
                parameters: serde_json::json!({}),
                estimated_cost: CostEstimate {
                    input_tokens: 0,
                    output_tokens: 0,
                },
            }],
            rationale: "test".into(),
        };
        let report = v.verify(&draft);
        assert!(!report.passed);
        assert_eq!(report.violations.len(), 1);
    }
}
