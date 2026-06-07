use disyn_core::ports::{
    ActionExecutor, FactExtractor, MemoryStore, ProposalEngine, RepairEngine, TelemetrySink,
    Verifier,
};
use disyn_core::types::{ApprovedPlan, ExecutionReport, Observation, PlanDraft};
use disyn_core::{Error, Result};
use disyn_runtime::BudgetManager;

const MAX_VERIFY_ITERATIONS: usize = 3;

pub struct Orchestrator {
    pub fact_extractor: Box<dyn FactExtractor>,
    pub proposal_engine: Box<dyn ProposalEngine>,
    pub verifier: Box<dyn Verifier>,
    pub repair_engine: Box<dyn RepairEngine>,
    pub memory: Box<dyn MemoryStore>,
    pub executor: Box<dyn ActionExecutor>,
    #[allow(dead_code)]
    pub telemetry: Box<dyn TelemetrySink>,
    pub budget: BudgetManager,
}

impl Orchestrator {
    pub async fn run(&mut self, obs: Observation) -> Result<ExecutionReport> {
        let facts = self.fact_extractor.extract(&obs).await?;
        let memory_ctx = self.memory.retrieve(&facts).await?;
        let mut draft = self.proposal_engine.propose(&facts, &memory_ctx).await?;

        let approved = match self.verify_loop(&mut draft) {
            Ok(plan) => plan,
            Err(_) if self.budget.can_replan() => {
                self.budget.record_replan();
                let mut redraft = self.proposal_engine.propose(&facts, &memory_ctx).await?;
                self.verify_loop(&mut redraft)?
            }
            Err(e) => return Err(e),
        };

        let report = self.executor.execute(&approved).await?;
        self.memory.persist(&report).await?;
        self.budget.record(&report.total_cost);
        Ok(report)
    }

    fn verify_loop(&mut self, draft: &mut PlanDraft) -> Result<ApprovedPlan> {
        for _ in 0..MAX_VERIFY_ITERATIONS {
            let report = self.verifier.verify(draft);
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
            match self.repair_engine.repair(draft, &report) {
                Some(fixed) => *draft = fixed,
                None => break,
            }
        }
        Err(Error::Verification {
            violations: self.verifier.verify(draft).violations.len(),
        })
    }
}
