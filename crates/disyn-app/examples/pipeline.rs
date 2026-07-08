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
            action: "verify:output-structure".into(),
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
        let results = plan
            .steps
            .iter()
            .enumerate()
            .map(|(i, step)| {
                println!("  [step {i}] {}", step.action);
                let output = if step.action == "verify:output-structure" {
                    serde_json::json!({
                        "verified": true,
                        "structure": {
                            "ok": "boolean"
                        }
                    })
                } else {
                    serde_json::json!({ "ok": true })
                };
                StepResult {
                    idempotency_key: step.idempotency_key,
                    step_index: i,
                    success: true,
                    output,
                    error: None,
                }
            })
            .collect();

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
