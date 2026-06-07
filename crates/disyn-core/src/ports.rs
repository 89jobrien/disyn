use async_trait::async_trait;

use crate::Result;
use crate::types::{
    ApprovedPlan, ExecutionReport, Facts, MemoryContext, Observation, PlanDraft, VerificationReport,
};

#[async_trait]
pub trait FactExtractor: Send + Sync {
    async fn extract(&self, observation: &Observation) -> Result<Facts>;
}

#[async_trait]
pub trait ProposalEngine: Send + Sync {
    async fn propose(&self, facts: &Facts, memory: &MemoryContext) -> Result<PlanDraft>;
}

pub trait Verifier: Send + Sync {
    fn verify(&self, draft: &PlanDraft) -> VerificationReport;
}

pub trait RepairEngine: Send + Sync {
    fn repair(&self, draft: &PlanDraft, report: &VerificationReport) -> Option<PlanDraft>;
}

#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn retrieve(&self, facts: &Facts) -> Result<MemoryContext>;
    async fn persist(&self, report: &ExecutionReport) -> Result<()>;
}

#[async_trait]
pub trait ActionExecutor: Send + Sync {
    async fn execute(&self, plan: &ApprovedPlan) -> Result<ExecutionReport>;
}

pub trait TelemetrySink: Send + Sync {
    fn emit(&self, event: &SpanEvent);
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
