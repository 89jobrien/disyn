//! Port traits that isolate each stage of the agent pipeline.

use async_trait::async_trait;

use crate::Result;
use crate::types::{
    ApprovedPlan, DeceptionReport, ExecutionReport, Facts, FormalSpec, GroundedExtraction,
    MemoryContext, Observation, PlanDraft, ProofVerdict, RetrievalStrategy, SymbolicAnchor,
    VerificationReport, WeightedPassage,
};

#[async_trait]
pub trait FactExtractor: Send + Sync {
    /// Extracts structured facts from an observation.
    async fn extract(&self, observation: &Observation) -> Result<Facts>;
}

#[async_trait]
pub trait ProposalEngine: Send + Sync {
    /// Proposes a plan from extracted facts and retrieved memory.
    async fn propose(&self, facts: &Facts, memory: &MemoryContext) -> Result<PlanDraft>;
}

pub trait Verifier: Send + Sync {
    /// Checks a draft plan and reports rule violations.
    fn verify(&self, draft: &PlanDraft) -> VerificationReport;
}

/// Implementations must preserve `PlannedStep::idempotency_key` for steps
/// with identical actions. New steps introduced by repair get fresh UUIDs.
pub trait RepairEngine: Send + Sync {
    /// Returns a repaired draft when the reported violations are repairable.
    fn repair(&self, draft: &PlanDraft, report: &VerificationReport) -> Option<PlanDraft>;
}

#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Retrieves memory relevant to the supplied facts.
    async fn retrieve(&self, facts: &Facts) -> Result<MemoryContext>;
    /// Persists a completed execution report for later retrieval.
    async fn persist(&self, report: &ExecutionReport) -> Result<()>;
}

#[async_trait]
pub trait ActionExecutor: Send + Sync {
    /// Executes an approved plan and reports each step's outcome.
    async fn execute(&self, plan: &ApprovedPlan) -> Result<ExecutionReport>;
}

#[async_trait]
pub trait GraphStore: Send + Sync {
    /// Adds extracted facts to the graph index.
    async fn index(&self, facts: &Facts) -> Result<()>;
    /// Retrieves weighted passages by traversing from the supplied facts.
    async fn traverse(
        &self,
        facts: &Facts,
        strategy: &RetrievalStrategy,
    ) -> Result<Vec<WeightedPassage>>;
    /// Resolves graph anchors for entities in the supplied facts.
    fn resolve_anchors(&self, facts: &Facts) -> Vec<SymbolicAnchor>;
}

pub trait TelemetrySink: Send + Sync {
    /// Emits one pipeline telemetry event.
    fn emit(&self, event: &SpanEvent);
}

pub trait FormalVerifier: Send + Sync {
    /// Synthesizes a formal safety specification from grounded facts.
    fn synthesize(&self, extraction: &GroundedExtraction) -> FormalSpec;
    /// Evaluates a formal specification against its grounded extraction.
    fn verify_spec(&self, spec: &FormalSpec, extraction: &GroundedExtraction) -> ProofVerdict;
    /// Detects deceptive evidence patterns in a grounded extraction.
    fn scan_deception(&self, _extraction: &GroundedExtraction) -> DeceptionReport {
        DeceptionReport {
            detected: false,
            level: None,
            evidence_fact_ids: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub enum SpanKind {
    ProposalGenerate,
    VerifierCheck,
    RepairApply,
    ExecutorDispatch,
}

#[derive(Debug, Clone)]
pub enum SpanStatus {
    Ok,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct SpanEvent {
    pub kind: SpanKind,
    pub trace_id: uuid::Uuid,
    pub parent_id: Option<uuid::Uuid>,
    pub duration_ms: u64,
    pub status: SpanStatus,
    pub metadata: serde_json::Value,
}
