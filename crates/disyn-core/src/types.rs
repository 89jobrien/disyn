use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum VerificationLayer {
    L0Format = 0,
    L1DataSource = 1,
    L2UserConstraints = 2,
    L3ToolContract = 3,
    L4Provenance = 4,
    L5Temporal = 5,
    L6Resource = 6,
    L7Semantic = 7,
    L8Mathematical = 8,
    L9Location = 9,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub source: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Facts {
    pub entities: Vec<String>,
    pub relations: Vec<(String, String, String)>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub observation: Observation,
    pub outcome: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContext {
    pub relevant_episodes: Vec<Episode>,
    pub summary: Option<String>,
    pub weighted_passages: Vec<WeightedPassage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CostClass {
    Symbolic,
    Neural,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub class: Option<CostClass>,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedStep {
    pub idempotency_key: uuid::Uuid,
    pub action: String,
    pub parameters: serde_json::Value,
    pub estimated_cost: CostEstimate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanDraft {
    pub steps: Vec<PlannedStep>,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Blocking,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub rule_id: String,
    pub layer: VerificationLayer,
    pub severity: Severity,
    pub message: String,
    pub step_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub passed: bool,
    pub violations: Vec<Violation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovedPlan {
    pub steps: Vec<PlannedStep>,
    pub verification: VerificationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub total_tokens: u64,
    pub symbolic_tokens: u64,
    pub neural_tokens: u64,
    pub wall_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub idempotency_key: uuid::Uuid,
    pub step_index: usize,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub results: Vec<StepResult>,
    pub total_cost: ResourceUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgNode {
    pub id: String,
    pub label: String,
    pub salience: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgEdge {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolicAnchor {
    pub entity_id: String,
    pub anchor_strength: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedPassage {
    pub content: String,
    pub source_episode_index: usize,
    pub triple_match_score: f32,
    pub final_weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalStrategy {
    pub max_hops: u8,
    pub top_k: usize,
    pub anchors: Vec<SymbolicAnchor>,
    pub edge_prune_threshold: f32,
    pub passage_boost: f32,
}

impl Default for RetrievalStrategy {
    fn default() -> Self {
        Self {
            max_hops: 3,
            top_k: 5,
            anchors: vec![],
            edge_prune_threshold: 0.1,
            passage_boost: 1.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_round_trips_through_serde() {
        let obs = Observation {
            source: "stdin".into(),
            payload: serde_json::json!({"query": "hello"}),
            timestamp: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&obs).unwrap();
        let back: Observation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.source, "stdin");
    }

    #[test]
    fn verification_report_passed_has_no_violations() {
        let report = VerificationReport {
            passed: true,
            violations: vec![],
        };
        assert!(report.passed);
        assert!(report.violations.is_empty());
    }
}
