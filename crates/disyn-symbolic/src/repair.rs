use disyn_core::ports::RepairEngine;
use disyn_core::types::{PlanDraft, PlannedStep, Severity, VerificationReport};

#[derive(Default)]
pub struct PatternRepairEngine;

impl RepairEngine for PatternRepairEngine {
    fn repair(&self, draft: &PlanDraft, report: &VerificationReport) -> Option<PlanDraft> {
        if report.violations.is_empty() {
            return None;
        }
        let blocking_indices: Vec<usize> = report
            .violations
            .iter()
            .filter(|v| matches!(v.severity, Severity::Blocking))
            .map(|v| v.step_index)
            .collect();
        let steps: Vec<PlannedStep> = draft
            .steps
            .iter()
            .enumerate()
            .filter(|(i, _)| !blocking_indices.contains(i))
            .map(|(_, s)| s.clone())
            .collect();
        Some(PlanDraft {
            steps,
            rationale: format!(
                "{} (repaired: removed {} blocking steps)",
                draft.rationale,
                blocking_indices.len()
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use disyn_core::types::{CostEstimate, Severity, VerificationLayer, Violation};
    use uuid::Uuid;

    #[test]
    fn removes_blocking_steps() {
        let engine = PatternRepairEngine;
        let draft = PlanDraft {
            steps: vec![
                PlannedStep {
                    idempotency_key: Uuid::nil(),
                    action: "".into(),
                    parameters: serde_json::json!({}),
                    estimated_cost: CostEstimate {
                        class: Some(disyn_core::types::CostClass::Symbolic),
                        input_tokens: 0,
                        output_tokens: 0,
                    },
                },
                PlannedStep {
                    idempotency_key: Uuid::nil(),
                    action: "echo hello".into(),
                    parameters: serde_json::json!({}),
                    estimated_cost: CostEstimate {
                        class: Some(disyn_core::types::CostClass::Symbolic),
                        input_tokens: 10,
                        output_tokens: 5,
                    },
                },
            ],
            rationale: "test".into(),
        };
        let report = VerificationReport {
            passed: false,
            violations: vec![Violation {
                rule_id: "non-empty-action".into(),
                layer: VerificationLayer::L0Format,
                severity: Severity::Blocking,
                message: "empty".into(),
                step_index: 0,
            }],
        };
        let fixed = engine.repair(&draft, &report).unwrap();
        assert_eq!(fixed.steps.len(), 1);
        assert_eq!(fixed.steps[0].action, "echo hello");
    }
}
