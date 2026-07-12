use std::time::Instant;

use disyn_core::ports::{
    ActionExecutor, FactExtractor, MemoryStore, ProposalEngine, RepairEngine, SpanEvent, SpanKind,
    SpanStatus, TelemetrySink, Verifier,
};
use disyn_core::types::{ApprovedPlan, ExecutionReport, Observation, PlanDraft};
use disyn_core::{Error, Result};
use disyn_runtime::BudgetManager;
use uuid::Uuid;

const MAX_VERIFY_ITERATIONS: usize = 3;

pub struct OrchestratorPorts {
    pub fact_extractor: Box<dyn FactExtractor>,
    pub proposal_engine: Box<dyn ProposalEngine>,
    pub verifier: Box<dyn Verifier>,
    pub repair_engine: Box<dyn RepairEngine>,
    pub memory: Box<dyn MemoryStore>,
    pub executor: Box<dyn ActionExecutor>,
    pub telemetry: Box<dyn TelemetrySink>,
}

pub struct Orchestrator {
    ports: OrchestratorPorts,
    budget: BudgetManager,
}

impl Orchestrator {
    pub fn new(ports: OrchestratorPorts, budget: BudgetManager) -> Self {
        Self { ports, budget }
    }

    pub async fn run(&mut self, obs: Observation) -> Result<ExecutionReport> {
        let trace_id = Uuid::new_v4();

        let facts = self.ports.fact_extractor.extract(&obs).await?;
        let memory_ctx = self.ports.memory.retrieve(&facts).await?;

        let t = Instant::now();
        let mut draft = self
            .ports
            .proposal_engine
            .propose(&facts, &memory_ctx)
            .await?;
        self.emit(
            SpanKind::ProposalGenerate,
            trace_id,
            t,
            serde_json::json!({"step_count": draft.steps.len()}),
        );

        let approved = match self.verify_loop(&mut draft, trace_id) {
            Ok(plan) => plan,
            Err(_) if self.budget.can_replan() => {
                self.budget.record_replan();
                let mut redraft = self
                    .ports
                    .proposal_engine
                    .propose(&facts, &memory_ctx)
                    .await?;
                self.verify_loop(&mut redraft, trace_id)?
            }
            Err(e) => return Err(e),
        };

        let t = Instant::now();
        let report = self.ports.executor.execute(&approved).await?;
        self.emit(
            SpanKind::ExecutorDispatch,
            trace_id,
            t,
            serde_json::json!({"step_count": approved.steps.len()}),
        );

        self.ports.memory.persist(&report).await?;
        self.budget.record(&report.total_cost);
        Ok(report)
    }

    fn emit(&self, kind: SpanKind, trace_id: Uuid, t: Instant, metadata: serde_json::Value) {
        self.ports.telemetry.emit(&SpanEvent {
            kind,
            trace_id,
            parent_id: None,
            duration_ms: t.elapsed().as_millis() as u64,
            status: SpanStatus::Ok,
            metadata,
        });
    }

    fn verify_loop(&mut self, draft: &mut PlanDraft, trace_id: Uuid) -> Result<ApprovedPlan> {
        let mut last_report = self.ports.verifier.verify(draft);
        for _ in 0..MAX_VERIFY_ITERATIONS {
            let t = Instant::now();
            self.emit(
                SpanKind::VerifierCheck,
                trace_id,
                t,
                serde_json::json!({"violations": last_report.violations.len(), "passed": last_report.passed}),
            );

            if last_report.passed {
                return Ok(ApprovedPlan {
                    steps: draft.steps.clone(),
                    verification: last_report,
                });
            }
            if !self.budget.can_repair() {
                break;
            }
            self.budget.record_repair();

            let t = Instant::now();
            let repaired = self.ports.repair_engine.repair(draft, &last_report);
            self.emit(
                SpanKind::RepairApply,
                trace_id,
                t,
                serde_json::json!({"applied": repaired.is_some()}),
            );

            match repaired {
                Some(fixed) => {
                    *draft = fixed;
                    last_report = self.ports.verifier.verify(draft);
                }
                None => break,
            }
        }
        Err(Error::Verification {
            violations: last_report.violations.len(),
            messages: last_report
                .violations
                .iter()
                .map(|v| v.message.clone())
                .collect(),
        })
    }
}
