//! End-to-end pipeline example using in-memory stubs — no API key required.
//!
//! Run with:
//!   cargo run --example pipeline

use async_trait::async_trait;
use chrono::Utc;
use disyn_app::orchestrator::Orchestrator;
use disyn_core::ports::{
    ActionExecutor, FactExtractor, MemoryStore, ProposalEngine, SpanEvent, TelemetrySink,
};
use disyn_core::types::{
    ApprovedPlan, CostClass, CostEstimate, Episode, ExecutionReport, Facts, MemoryContext,
    Observation, PlanDraft, PlannedStep, ResourceUsage, StepResult,
};
use disyn_core::{Error, Result};
use disyn_runtime::BudgetManager;
use disyn_symbolic::{PatternRepairEngine, RuleSetVerifier};
const VERIFY_OUTPUT_STRUCTURE_ACTION: &str = "verify:output-structure";

// --- Stub FactExtractor ---------------------------------------------------

struct EchoExtractor;

#[async_trait]
impl FactExtractor for EchoExtractor {
    async fn extract(&self, observation: &Observation) -> Result<Facts> {
        let entities = observation
            .payload
            .as_object()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        Ok(Facts {
            entities,
            relations: vec![],
            confidence: 1.0,
        })
    }
}

// --- Stub ProposalEngine --------------------------------------------------

struct EchoProposal;

#[async_trait]
impl ProposalEngine for EchoProposal {
    async fn propose(&self, facts: &Facts, memory: &MemoryContext) -> Result<PlanDraft> {
        let prior = memory
            .summary
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|s| format!(" (prior context: {s})"))
            .unwrap_or_default();

        let mut steps = facts
            .entities
            .iter()
            .map(|e| PlannedStep {
                idempotency_key: uuid::Uuid::new_v4(),
                action: format!("process:{e}"),
                parameters: serde_json::json!({ "entity": e }),
                estimated_cost: CostEstimate {
                    class: Some(CostClass::Symbolic),
                    input_tokens: 10,
                    output_tokens: 5,
                },
            })
            .collect::<Vec<_>>();

        steps.push(PlannedStep {
            idempotency_key: uuid::Uuid::new_v4(),
            action: VERIFY_OUTPUT_STRUCTURE_ACTION.into(),
            parameters: serde_json::json!({
                "expected": {
                    "ok": "boolean"
                }
            }),
            estimated_cost: CostEstimate {
                class: Some(CostClass::Symbolic),
                input_tokens: 5,
                output_tokens: 0,
            },
        });

        Ok(PlanDraft {
            steps,
            rationale: format!("echo plan for {:?}{prior}", facts.entities),
        })
    }
}

// --- Stub MemoryStore (returns a canned summary) -------------------------

struct CannedMemory {
    summary: String,
}

#[async_trait]
impl MemoryStore for CannedMemory {
    async fn retrieve(&self, _facts: &Facts) -> Result<MemoryContext> {
        Ok(MemoryContext {
            relevant_episodes: vec![Episode {
                observation: Observation {
                    source: "history".into(),
                    payload: serde_json::json!({}),
                    timestamp: Utc::now(),
                },
                outcome: Some("ok".into()),
                timestamp: Utc::now(),
            }],
            summary: Some(self.summary.clone()),
            weighted_passages: vec![],
        })
    }

    async fn persist(&self, _report: &ExecutionReport) -> Result<()> {
        Ok(())
    }
}

// --- Stub ActionExecutor -------------------------------------------------

struct PrintExecutor;

#[async_trait]
impl ActionExecutor for PrintExecutor {
    async fn execute(&self, plan: &ApprovedPlan) -> Result<ExecutionReport> {
        let mut results = Vec::new();

        for (i, step) in plan.steps.iter().enumerate() {
            println!("  [step {i}] {}", step.action);
            let result = if step.action == VERIFY_OUTPUT_STRUCTURE_ACTION {
                match verify_prior_output_structure(step, &results) {
                    Ok(output) => successful_step_result(step, i, output),
                    Err(error) => StepResult {
                        idempotency_key: step.idempotency_key,
                        step_index: i,
                        success: false,
                        output: serde_json::json!({ "verified": false }),
                        error: Some(error),
                    },
                }
            } else {
                successful_step_result(step, i, serde_json::json!({ "ok": true }))
            };
            results.push(result);
        }

        Ok(ExecutionReport {
            results,
            total_cost: ResourceUsage {
                total_tokens: 50,
                symbolic_tokens: 50,
                neural_tokens: 0,
                wall_time_ms: 1,
            },
        })
    }
}

fn successful_step_result(
    step: &PlannedStep,
    step_index: usize,
    output: serde_json::Value,
) -> StepResult {
    StepResult {
        idempotency_key: step.idempotency_key,
        step_index,
        success: true,
        output,
        error: None,
    }
}

fn verify_prior_output_structure(
    step: &PlannedStep,
    prior_results: &[StepResult],
) -> std::result::Result<serde_json::Value, String> {
    if prior_results.is_empty() {
        return Err("no prior step outputs to verify".into());
    }

    let expected = step
        .parameters
        .get("expected")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "verification step is missing an `expected` object".to_string())?;

    for result in prior_results {
        if !result.success {
            return Err(format!(
                "cannot verify output because step {} failed",
                result.step_index
            ));
        }

        verify_output_matches_schema(result.step_index, &result.output, expected)?;
    }

    Ok(serde_json::json!({
        "verified": true,
        "checked_steps": prior_results.len(),
        "structure": step.parameters["expected"].clone()
    }))
}

fn verify_output_matches_schema(
    step_index: usize,
    output: &serde_json::Value,
    expected: &serde_json::Map<String, serde_json::Value>,
) -> std::result::Result<(), String> {
    let object = output
        .as_object()
        .ok_or_else(|| format!("step {step_index} output must be an object"))?;

    for (field, expected_type) in expected {
        let expected_type = expected_type.as_str().ok_or_else(|| {
            format!("expected schema entry for field `{field}` must be a string type name")
        })?;
        let value = object.get(field).ok_or_else(|| {
            format!("step {step_index} output is missing required field `{field}`")
        })?;

        if !value_matches_type(value, expected_type) {
            return Err(format!(
                "step {step_index} output field `{field}` expected {expected_type}, got {}",
                describe_json_type(value)
            ));
        }
    }

    Ok(())
}

fn value_matches_type(value: &serde_json::Value, expected_type: &str) -> bool {
    match expected_type {
        "array" => value.is_array(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        "number" => value.is_number(),
        "object" => value.is_object(),
        "string" => value.is_string(),
        _ => false,
    }
}

fn describe_json_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Null => "null",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Object(_) => "object",
        serde_json::Value::String(_) => "string",
    }
}
fn verify_execution_report(report: &ExecutionReport) -> Result<()> {
    const EXPECTED_STEP_COUNT: usize = 4;

    if report.results.len() != EXPECTED_STEP_COUNT {
        return Err(Error::Other(format!(
            "expected {EXPECTED_STEP_COUNT} execution results, got {}",
            report.results.len()
        )));
    }

    for (expected_index, result) in report.results.iter().enumerate() {
        if result.step_index != expected_index {
            return Err(Error::Other(format!(
                "expected step index {expected_index}, got {}",
                result.step_index
            )));
        }

        if !result.success {
            return Err(Error::Other(format!(
                "step {expected_index} was not successful"
            )));
        }

        let expected_output = if expected_index == EXPECTED_STEP_COUNT - 1 {
            serde_json::json!({
                "verified": true,
                "checked_steps": 3,
                "structure": {
                    "ok": "boolean"
                }
            })
        } else {
            serde_json::json!({ "ok": true })
        };

        if result.output != expected_output {
            return Err(Error::Other(format!(
                "step {expected_index} had unexpected output: {}",
                result.output
            )));
        }

        if let Some(error) = &result.error {
            return Err(Error::Other(format!(
                "step {expected_index} returned unexpected error: {error}"
            )));
        }
    }

    if report.total_cost.total_tokens != 50
        || report.total_cost.symbolic_tokens != 50
        || report.total_cost.neural_tokens != 0
        || report.total_cost.wall_time_ms != 1
    {
        return Err(Error::Other(format!(
            "unexpected total cost: {}",
            serde_json::to_string(&report.total_cost).map_err(|e| Error::Other(e.to_string()))?
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verification_step() -> PlannedStep {
        PlannedStep {
            idempotency_key: uuid::Uuid::nil(),
            action: VERIFY_OUTPUT_STRUCTURE_ACTION.into(),
            parameters: serde_json::json!({
                "expected": {
                    "ok": "boolean"
                }
            }),
            estimated_cost: CostEstimate {
                class: Some(CostClass::Symbolic),
                input_tokens: 0,
                output_tokens: 0,
            },
        }
    }

    fn prior_result(output: serde_json::Value) -> StepResult {
        StepResult {
            idempotency_key: uuid::Uuid::nil(),
            step_index: 0,
            success: true,
            output,
            error: None,
        }
    }

    #[test]
    fn output_structure_verifier_accepts_expected_shape() {
        let output = verify_prior_output_structure(
            &verification_step(),
            &[prior_result(serde_json::json!({ "ok": true }))],
        )
        .unwrap();

        assert_eq!(
            output,
            serde_json::json!({
                "verified": true,
                "checked_steps": 1,
                "structure": {
                    "ok": "boolean"
                }
            })
        );
    }

    #[test]
    fn output_structure_verifier_rejects_wrong_field_type() {
        let error = verify_prior_output_structure(
            &verification_step(),
            &[prior_result(serde_json::json!({ "ok": "true" }))],
        )
        .unwrap_err();

        assert_eq!(
            error,
            "step 0 output field `ok` expected boolean, got string"
        );
    }

    #[test]
    fn output_structure_verifier_rejects_missing_field() {
        let error = verify_prior_output_structure(
            &verification_step(),
            &[prior_result(serde_json::json!({ "status": "ok" }))],
        )
        .unwrap_err();

        assert_eq!(error, "step 0 output is missing required field `ok`");
    }
}
// --- Null TelemetrySink --------------------------------------------------

struct NullSink;

impl TelemetrySink for NullSink {
    fn emit(&self, _event: &SpanEvent) {}
}

// -------------------------------------------------------------------------

#[tokio::main]
async fn main() -> disyn_core::Result<()> {
    let obs = Observation {
        source: "example".into(),
        payload: serde_json::json!({
            "user": "alice",
            "action": "checkout",
            "item": "book"
        }),
        timestamp: Utc::now(),
    };

    let mut orchestrator = Orchestrator::new(
        Box::new(EchoExtractor),
        Box::new(EchoProposal),
        Box::new(RuleSetVerifier::default()),
        Box::new(PatternRepairEngine),
        Box::new(CannedMemory {
            summary: "alice prefers express shipping".into(),
        }),
        Box::new(PrintExecutor),
        Box::new(NullSink),
        BudgetManager::new(10_000, 1, 3, 10_000),
    );

    println!("Running pipeline...");
    let report = orchestrator.run(obs).await?;
    println!("\nExecution report:");
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|e| disyn_core::Error::Other(e.to_string()))?
    );

    println!("\nVerifying execution report...");
    verify_execution_report(&report)?;
    println!("Execution report verified.");
    Ok(())
}
