use std::collections::BTreeMap;

use disyn_core::ports::Verifier;
use disyn_core::types::{PlanDraft, Severity, VerificationLayer, VerificationReport, Violation};

use crate::rules::LayerRule;

pub struct NonEmptyActionRule;

impl LayerRule for NonEmptyActionRule {
    fn layer(&self) -> VerificationLayer {
        VerificationLayer::L0Format
    }

    fn rule_id(&self) -> &str {
        "non-empty-action"
    }

    fn check(&self, draft: &PlanDraft) -> Vec<Violation> {
        draft
            .steps
            .iter()
            .enumerate()
            .filter(|(_, step)| step.action.is_empty())
            .map(|(i, _)| Violation {
                rule_id: self.rule_id().into(),
                layer: self.layer(),
                severity: Severity::Blocking,
                message: "step action must not be empty".into(),
                step_index: i,
            })
            .collect()
    }
}

pub struct RuleSetVerifier {
    rules: BTreeMap<VerificationLayer, Vec<Box<dyn LayerRule>>>,
}

impl RuleSetVerifier {
    pub fn new() -> Self {
        let mut v = Self {
            rules: BTreeMap::new(),
        };
        v.register(NonEmptyActionRule);
        v
    }

    pub fn register(&mut self, rule: impl LayerRule + 'static) {
        self.rules.entry(rule.layer()).or_default().push(Box::new(rule));
    }
}

impl Default for RuleSetVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Verifier for RuleSetVerifier {
    fn verify(&self, draft: &PlanDraft) -> VerificationReport {
        let mut violations = Vec::new();
        for rules in self.rules.values() {
            for rule in rules {
                violations.extend(rule.check(draft));
            }
            if violations
                .iter()
                .any(|v| matches!(v.severity, Severity::Blocking))
            {
                break;
            }
        }
        VerificationReport {
            passed: !violations
                .iter()
                .any(|v| matches!(v.severity, Severity::Blocking)),
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
        let v = RuleSetVerifier::default();
        let draft = PlanDraft {
            steps: vec![],
            rationale: "no-op".into(),
        };
        let report = v.verify(&draft);
        assert!(report.passed);
    }

    #[test]
    fn step_with_empty_action_fails() {
        let v = RuleSetVerifier::default();
        let draft = PlanDraft {
            steps: vec![PlannedStep {
                action: "".into(),
                parameters: serde_json::json!({}),
                estimated_cost: CostEstimate {
                    class: Some(disyn_core::types::CostClass::Symbolic),
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
