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

pub struct Orchestrator {
    pub fact_extractor: Box<dyn FactExtractor>,
    pub proposal_engine: Box<dyn ProposalEngine>,
    pub verifier: Box<dyn Verifier>,
    pub repair_engine: Box<dyn RepairEngine>,
    pub memory: Box<dyn MemoryStore>,
    pub executor: Box<dyn ActionExecutor>,
    pub telemetry: Box<dyn TelemetrySink>,
    pub budget: BudgetManager,
}

impl Orchestrator {
    pub async fn run(&mut self, obs: Observation) -> Result<ExecutionReport> {
        let trace_id = Uuid::new_v4();

        let facts = self.fact_extractor.extract(&obs).await?;
        let memory_ctx = self.memory.retrieve(&facts).await?;

        let t = Instant::now();
        let mut draft = self.proposal_engine.propose(&facts, &memory_ctx).await?;
        self.telemetry.emit(&SpanEvent {
            kind: SpanKind::ProposalGenerate,
            trace_id,
            parent_id: None,
            duration_ms: t.elapsed().as_millis() as u64,
            status: SpanStatus::Ok,
            metadata: serde_json::json!({"step_count": draft.steps.len()}),
        });

        let approved = match self.verify_loop(&mut draft, trace_id) {
            Ok(plan) => plan,
            Err(_) if self.budget.can_replan() => {
                self.budget.record_replan();
                let mut redraft = self.proposal_engine.propose(&facts, &memory_ctx).await?;
                self.verify_loop(&mut redraft, trace_id)?
            }
            Err(e) => return Err(e),
        };

        let t = Instant::now();
        let report = self.executor.execute(&approved).await?;
        self.telemetry.emit(&SpanEvent {
            kind: SpanKind::ExecutorDispatch,
            trace_id,
            parent_id: None,
            duration_ms: t.elapsed().as_millis() as u64,
            status: SpanStatus::Ok,
            metadata: serde_json::json!({"step_count": approved.steps.len()}),
        });

        self.memory.persist(&report).await?;
        self.budget.record(&report.total_cost);
        Ok(report)
    }

    fn verify_loop(&mut self, draft: &mut PlanDraft, trace_id: Uuid) -> Result<ApprovedPlan> {
        for _ in 0..MAX_VERIFY_ITERATIONS {
            let t = Instant::now();
            let report = self.verifier.verify(draft);
            self.telemetry.emit(&SpanEvent {
                kind: SpanKind::VerifierCheck,
                trace_id,
                parent_id: None,
                duration_ms: t.elapsed().as_millis() as u64,
                status: SpanStatus::Ok,
                metadata: serde_json::json!({"violations": report.violations.len(), "passed": report.passed}),
            });

            if report.passed {
                return Ok(ApprovedPlan {
                    steps: draft.steps.clone(),
                    verification: report,
                });
            }
            if !self.budget.can_repair() {
                break;
            }
            self.budget.record_repair();

            let t = Instant::now();
            let repaired = self.repair_engine.repair(draft, &report);
            self.telemetry.emit(&SpanEvent {
                kind: SpanKind::RepairApply,
                trace_id,
                parent_id: None,
                duration_ms: t.elapsed().as_millis() as u64,
                status: SpanStatus::Ok,
                metadata: serde_json::json!({"applied": repaired.is_some()}),
            });

            match repaired {
                Some(fixed) => *draft = fixed,
                None => break,
            }
        }
        // TODO: Include the specific violation messages in the error so callers can surface which
        // rules failed rather than just the count.
        Err(Error::Verification {
            violations: self.verifier.verify(draft).violations.len(),
        })
    }
}
