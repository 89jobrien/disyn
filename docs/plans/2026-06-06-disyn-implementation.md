# Plan: disyn — Hybrid Agent System

## Goal

Scaffold and implement a 7-crate Rust workspace that transforms raw
observations into verified actions through a typed pipeline with symbolic
repair gating neural replans.

## Architecture

- Crates affected: all 7 (new workspace)
- New traits/types: all port traits in disyn-core, DTOs in disyn-core::types
- Data flow: Observation -> Facts -> MemoryContext -> PlanDraft ->
  VerificationReport -> ApprovedPlan -> ExecutionReport

## Tech Stack

- Rust edition 2024, resolver 3, MSRV 1.88
- Key deps: tokio, async-trait, serde/serde_json, thiserror, tracing,
  uuid, chrono, reqwest (neural), clap (app)
- License: MIT OR Apache-2.0

## Tasks

### Task 1: Workspace scaffold

**Crate**: (root)
**File(s)**: `Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`,
`crates/disyn-core/Cargo.toml`, `crates/disyn-core/src/lib.rs`,
`crates/disyn-symbolic/Cargo.toml`, `crates/disyn-symbolic/src/lib.rs`,
`crates/disyn-neural/Cargo.toml`, `crates/disyn-neural/src/lib.rs`,
`crates/disyn-memory/Cargo.toml`, `crates/disyn-memory/src/lib.rs`,
`crates/disyn-runtime/Cargo.toml`, `crates/disyn-runtime/src/lib.rs`,
`crates/disyn-app/Cargo.toml`, `crates/disyn-app/src/main.rs`,
`crates/disyn-xtask/Cargo.toml`, `crates/disyn-xtask/src/main.rs`
**Run**: `cargo check --workspace`

1. Create root `Cargo.toml`:

   ```toml
   [workspace]
   members = [
       "crates/disyn-core",
       "crates/disyn-symbolic",
       "crates/disyn-neural",
       "crates/disyn-memory",
       "crates/disyn-runtime",
       "crates/disyn-app",
   ]
   exclude = ["crates/disyn-xtask"]
   resolver = "3"

   [workspace.package]
   version = "0.1.0"
   edition = "2024"
   rust-version = "1.88"
   license = "MIT OR Apache-2.0"

   [workspace.dependencies]
   async-trait = "0.1"
   chrono = { version = "0.4", features = ["serde"] }
   clap = { version = "4", features = ["derive"] }
   reqwest = { version = "0.12", features = ["json"] }
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   thiserror = "2"
   tokio = { version = "1", features = ["full"] }
   tracing = "0.1"
   tracing-subscriber = { version = "0.3", features = ["json"] }
   uuid = { version = "1", features = ["v4", "serde"] }
   ```

2. Create `rust-toolchain.toml`:

   ```toml
   [toolchain]
   channel = "1.88"
   components = ["clippy", "rustfmt"]
   ```

3. Create `.cargo/config.toml`:

   ```toml
   [alias]
   xtask = "run --manifest-path crates/disyn-xtask/Cargo.toml --"

   [build]
   rustflags = ["-D", "warnings"]
   ```

4. Create each crate's `Cargo.toml` with `version.workspace = true`,
   `edition.workspace = true`, `license.workspace = true`. Each `src/lib.rs`
   or `src/main.rs` starts as a stub (empty lib or `fn main() {}`).

   `disyn-core` deps: `async-trait`, `chrono`, `serde`, `serde_json`,
   `thiserror`, `uuid`.

   `disyn-symbolic` deps: `disyn-core = { path = "../disyn-core" }`.

   `disyn-neural` deps: `disyn-core`, `async-trait`, `reqwest`,
   `serde`, `serde_json`, `tokio`.

   `disyn-memory` deps: `disyn-core`, `async-trait`, `tokio`.

   `disyn-runtime` deps: `disyn-core`, `async-trait`, `tokio`,
   `tracing`, `uuid`.

   `disyn-app` deps: `disyn-core`, `disyn-symbolic`, `disyn-neural`,
   `disyn-memory`, `disyn-runtime`, `clap`, `tokio`, `serde_json`,
   `tracing`, `tracing-subscriber`.

   `disyn-xtask` deps: none (uses `std::process::Command`).

5. Verify:

   ```
   cargo check --workspace          → compiles
   cargo clippy --workspace -- -D warnings  → zero warnings
   ```

6. Commit: `git commit -m "chore: scaffold disyn workspace (7 crates)"`

---

### Task 2: Core error type

**Crate**: `disyn-core`
**File(s)**: `crates/disyn-core/src/error.rs`, `crates/disyn-core/src/lib.rs`
**Run**: `cargo check -p disyn-core`

1. Write `error.rs`:

   ```rust
   use thiserror::Error;

   #[derive(Debug, Error)]
   pub enum Error {
       #[error("inference failed: {0}")]
       Inference(String),

       #[error("verification failed: {violations} violations")]
       Verification { violations: usize },

       #[error("repair exhausted after {attempts} attempts")]
       RepairExhausted { attempts: u32 },

       #[error("budget exceeded: {0}")]
       BudgetExceeded(String),

       #[error("memory store: {0}")]
       Memory(String),

       #[error("execution failed: {0}")]
       Execution(String),

       #[error("{0}")]
       Other(String),
   }

   pub type Result<T> = std::result::Result<T, Error>;
   ```

2. Add `pub mod error;` and `pub use error::{Error, Result};` to `lib.rs`.

3. Verify: `cargo check -p disyn-core`

4. Commit: `git commit -m "feat(core): add Error type and Result alias"`

---

### Task 3: Core DTOs

**Crate**: `disyn-core`
**File(s)**: `crates/disyn-core/src/types.rs`, `crates/disyn-core/src/lib.rs`
**Run**: `cargo test -p disyn-core`

1. Write failing test in `types.rs`:

   ```rust
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
   ```

2. Implement all DTOs in `types.rs`:

   ```rust
   use chrono::{DateTime, Utc};
   use serde::{Deserialize, Serialize};

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
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct CostEstimate {
       pub input_tokens: u32,
       pub output_tokens: u32,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct PlannedStep {
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
       pub wall_time_ms: u64,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct StepResult {
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
   ```

3. Add `pub mod types;` to `lib.rs`.

4. Verify:

   ```
   cargo test -p disyn-core        → 2 tests pass
   cargo clippy -p disyn-core      → zero warnings
   ```

5. Commit: `git commit -m "feat(core): add pipeline DTOs with serde"`

---

### Task 4: Port traits

**Crate**: `disyn-core`
**File(s)**: `crates/disyn-core/src/ports.rs`, `crates/disyn-core/src/lib.rs`
**Run**: `cargo check -p disyn-core`

1. Write `ports.rs`:

   ```rust
   use async_trait::async_trait;

   use crate::types::*;
   use crate::Result;

   // --- Neural ports (async) ---

   #[async_trait]
   pub trait FactExtractor: Send + Sync {
       async fn extract(&self, observation: &Observation) -> Result<Facts>;
   }

   #[async_trait]
   pub trait ProposalEngine: Send + Sync {
       async fn propose(
           &self,
           facts: &Facts,
           memory: &MemoryContext,
       ) -> Result<PlanDraft>;
   }

   // --- Symbolic ports (sync, deterministic) ---

   pub trait Verifier: Send + Sync {
       fn verify(&self, draft: &PlanDraft) -> VerificationReport;
   }

   pub trait RepairEngine: Send + Sync {
       fn repair(
           &self,
           draft: &PlanDraft,
           report: &VerificationReport,
       ) -> Option<PlanDraft>;
   }

   // --- Memory port (async) ---

   #[async_trait]
   pub trait MemoryStore: Send + Sync {
       async fn retrieve(&self, facts: &Facts) -> Result<MemoryContext>;
       async fn persist(&self, report: &ExecutionReport) -> Result<()>;
   }

   // --- Runtime ports ---

   #[async_trait]
   pub trait ActionExecutor: Send + Sync {
       async fn execute(
           &self,
           plan: &ApprovedPlan,
       ) -> Result<ExecutionReport>;
   }

   pub trait TelemetrySink: Send + Sync {
       fn emit(&self, event: &SpanEvent);
   }

   // --- Telemetry types ---

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
   ```

2. Add `pub mod ports;` to `lib.rs`.

3. Verify: `cargo check -p disyn-core`

4. Commit: `git commit -m "feat(core): add port traits and telemetry types"`

---

### Task 5: Symbolic crate — Verifier and RepairEngine

**Crate**: `disyn-symbolic`
**File(s)**: `crates/disyn-symbolic/src/verifier.rs`,
`crates/disyn-symbolic/src/repair.rs`, `crates/disyn-symbolic/src/lib.rs`
**Run**: `cargo test -p disyn-symbolic`

1. Write failing tests in `verifier.rs`:

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use disyn_core::types::*;

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
   ```

2. Implement `RuleSetVerifier`:

   ```rust
   use disyn_core::ports::Verifier;
   use disyn_core::types::*;

   #[derive(Default)]
   pub struct RuleSetVerifier;

   impl Verifier for RuleSetVerifier {
       fn verify(&self, draft: &PlanDraft) -> VerificationReport {
           let mut violations = Vec::new();
           for (i, step) in draft.steps.iter().enumerate() {
               if step.action.is_empty() {
                   violations.push(Violation {
                       rule_id: "non-empty-action".into(),
                       severity: Severity::Blocking,
                       message: "step action must not be empty".into(),
                       step_index: i,
                   });
               }
           }
           VerificationReport {
               passed: violations.is_empty(),
               violations,
           }
       }
   }
   ```

3. Write `repair.rs` with test:

   ```rust
   use disyn_core::ports::RepairEngine;
   use disyn_core::types::*;

   #[derive(Default)]
   pub struct PatternRepairEngine;

   impl RepairEngine for PatternRepairEngine {
       fn repair(
           &self,
           draft: &PlanDraft,
           report: &VerificationReport,
       ) -> Option<PlanDraft> {
           if report.violations.is_empty() {
               return None;
           }
           // Remove steps with blocking violations
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

       #[test]
       fn removes_blocking_steps() {
           let engine = PatternRepairEngine;
           let draft = PlanDraft {
               steps: vec![
                   PlannedStep {
                       action: "".into(),
                       parameters: serde_json::json!({}),
                       estimated_cost: CostEstimate {
                           input_tokens: 0,
                           output_tokens: 0,
                       },
                   },
                   PlannedStep {
                       action: "echo hello".into(),
                       parameters: serde_json::json!({}),
                       estimated_cost: CostEstimate {
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
   ```

4. Update `lib.rs`:

   ```rust
   pub mod repair;
   pub mod verifier;

   pub use repair::PatternRepairEngine;
   pub use verifier::RuleSetVerifier;
   ```

5. Verify:

   ```
   cargo test -p disyn-symbolic     → 3 tests pass
   cargo clippy -p disyn-symbolic   → zero warnings
   ```

6. Commit: `git commit -m "feat(symbolic): add RuleSetVerifier and PatternRepairEngine"`

---

### Task 6: Memory crate — InMemoryStore

**Crate**: `disyn-memory`
**File(s)**: `crates/disyn-memory/src/in_memory.rs`,
`crates/disyn-memory/src/lib.rs`
**Run**: `cargo test -p disyn-memory`

1. Write failing test:

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use disyn_core::types::*;

       #[tokio::test]
       async fn retrieve_returns_empty_context_initially() {
           let store = InMemoryStore::new();
           let facts = Facts {
               entities: vec!["test".into()],
               relations: vec![],
               confidence: 1.0,
           };
           let ctx = store.retrieve(&facts).await.unwrap();
           assert!(ctx.relevant_episodes.is_empty());
           assert!(ctx.summary.is_none());
       }
   }
   ```

2. Implement:

   ```rust
   use async_trait::async_trait;
   use std::sync::Mutex;

   use disyn_core::ports::MemoryStore;
   use disyn_core::types::*;
   use disyn_core::Result;

   pub struct InMemoryStore {
       episodes: Mutex<Vec<ExecutionReport>>,
   }

   impl InMemoryStore {
       pub fn new() -> Self {
           Self {
               episodes: Mutex::new(Vec::new()),
           }
       }
   }

   #[async_trait]
   impl MemoryStore for InMemoryStore {
       async fn retrieve(&self, _facts: &Facts) -> Result<MemoryContext> {
           Ok(MemoryContext {
               relevant_episodes: vec![],
               summary: None,
           })
       }

       async fn persist(&self, report: &ExecutionReport) -> Result<()> {
           self.episodes
               .lock()
               .map_err(|e| disyn_core::Error::Memory(e.to_string()))?
               .push(report.clone());
           Ok(())
       }
   }
   ```

3. Update `lib.rs`: `pub mod in_memory; pub use in_memory::InMemoryStore;`

4. Add `tokio` as dev-dependency for tests.

5. Verify:

   ```
   cargo test -p disyn-memory      → 1 test passes
   cargo clippy -p disyn-memory    → zero warnings
   ```

6. Commit: `git commit -m "feat(memory): add InMemoryStore"`

---

### Task 7: Neural crate — OpenAI adapter stubs

**Crate**: `disyn-neural`
**File(s)**: `crates/disyn-neural/src/openai.rs`,
`crates/disyn-neural/src/ollama.rs`, `crates/disyn-neural/src/lib.rs`
**Run**: `cargo test -p disyn-neural`

1. Write test with mock (no live API):

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;
       use disyn_core::types::*;

       #[tokio::test]
       async fn openai_extractor_requires_api_key() {
           let config = OpenAiConfig {
               api_key: String::new(),
               model: "gpt-4o".into(),
               base_url: "https://api.openai.com/v1".into(),
           };
           let extractor = OpenAiFactExtractor::new(config);
           let obs = Observation {
               source: "test".into(),
               payload: serde_json::json!({"q": "hi"}),
               timestamp: chrono::Utc::now(),
           };
           let result = extractor.extract(&obs).await;
           assert!(result.is_err());
       }
   }
   ```

2. Implement `OpenAiConfig`, `OpenAiFactExtractor`, `OpenAiProposalEngine`:

   ```rust
   use async_trait::async_trait;
   use disyn_core::ports::{FactExtractor, ProposalEngine};
   use disyn_core::types::*;
   use disyn_core::{Error, Result};

   #[derive(Clone)]
   pub struct OpenAiConfig {
       pub api_key: String,
       pub model: String,
       pub base_url: String,
   }

   pub struct OpenAiFactExtractor {
       config: OpenAiConfig,
       client: reqwest::Client,
   }

   impl OpenAiFactExtractor {
       pub fn new(config: OpenAiConfig) -> Self {
           Self {
               config,
               client: reqwest::Client::new(),
           }
       }
   }

   #[async_trait]
   impl FactExtractor for OpenAiFactExtractor {
       async fn extract(&self, observation: &Observation) -> Result<Facts> {
           if self.config.api_key.is_empty() {
               return Err(Error::Inference(
                   "OPENAI_API_KEY not set".into(),
               ));
           }
           let body = serde_json::json!({
               "model": self.config.model,
               "messages": [{
                   "role": "user",
                   "content": format!(
                       "Extract entities and relations from: {}",
                       observation.payload
                   ),
               }],
               "response_format": { "type": "json_object" },
           });
           let resp = self
               .client
               .post(format!(
                   "{}/chat/completions",
                   self.config.base_url
               ))
               .bearer_auth(&self.config.api_key)
               .json(&body)
               .send()
               .await
               .map_err(|e| Error::Inference(e.to_string()))?;
           let data: serde_json::Value = resp
               .json()
               .await
               .map_err(|e| Error::Inference(e.to_string()))?;
           // Parse LLM response into Facts
           let content = data["choices"][0]["message"]["content"]
               .as_str()
               .unwrap_or("{}");
           let parsed: serde_json::Value = serde_json::from_str(content)
               .unwrap_or(serde_json::json!({}));
           Ok(Facts {
               entities: parsed["entities"]
                   .as_array()
                   .map(|a| {
                       a.iter()
                           .filter_map(|v| v.as_str().map(String::from))
                           .collect()
                   })
                   .unwrap_or_default(),
               relations: vec![],
               confidence: 0.5,
           })
       }
   }

   pub struct OpenAiProposalEngine {
       config: OpenAiConfig,
       client: reqwest::Client,
   }

   impl OpenAiProposalEngine {
       pub fn new(config: OpenAiConfig) -> Self {
           Self {
               config,
               client: reqwest::Client::new(),
           }
       }
   }

   #[async_trait]
   impl ProposalEngine for OpenAiProposalEngine {
       async fn propose(
           &self,
           facts: &Facts,
           _memory: &MemoryContext,
       ) -> Result<PlanDraft> {
           if self.config.api_key.is_empty() {
               return Err(Error::Inference(
                   "OPENAI_API_KEY not set".into(),
               ));
           }
           let body = serde_json::json!({
               "model": self.config.model,
               "messages": [{
                   "role": "user",
                   "content": format!(
                       "Given these facts, propose a plan: {:?}",
                       facts.entities
                   ),
               }],
               "response_format": { "type": "json_object" },
           });
           let resp = self
               .client
               .post(format!(
                   "{}/chat/completions",
                   self.config.base_url
               ))
               .bearer_auth(&self.config.api_key)
               .json(&body)
               .send()
               .await
               .map_err(|e| Error::Inference(e.to_string()))?;
           let data: serde_json::Value = resp
               .json()
               .await
               .map_err(|e| Error::Inference(e.to_string()))?;
           let content = data["choices"][0]["message"]["content"]
               .as_str()
               .unwrap_or("{}");
           let parsed: serde_json::Value = serde_json::from_str(content)
               .unwrap_or(serde_json::json!({}));
           let steps = parsed["steps"]
               .as_array()
               .map(|arr| {
                   arr.iter()
                       .map(|s| PlannedStep {
                           action: s["action"]
                               .as_str()
                               .unwrap_or("unknown")
                               .to_string(),
                           parameters: s["parameters"].clone(),
                           estimated_cost: CostEstimate {
                               input_tokens: 0,
                               output_tokens: 0,
                           },
                       })
                       .collect()
               })
               .unwrap_or_default();
           Ok(PlanDraft {
               steps,
               rationale: parsed["rationale"]
                   .as_str()
                   .unwrap_or("LLM-generated plan")
                   .to_string(),
           })
       }
   }
   ```

3. Add `OllamaFactExtractor` and `OllamaProposalEngine` in `ollama.rs`
   — same structure as OpenAI but with
   `base_url: "http://localhost:11434/v1"` and no auth header.

4. Update `lib.rs`:

   ```rust
   pub mod ollama;
   pub mod openai;

   pub use ollama::{OllamaFactExtractor, OllamaProposalEngine};
   pub use openai::{
       OpenAiConfig, OpenAiFactExtractor, OpenAiProposalEngine,
   };
   ```

5. Verify:

   ```
   cargo test -p disyn-neural      → 1 test passes
   cargo clippy -p disyn-neural    → zero warnings
   ```

6. Commit: `git commit -m "feat(neural): add OpenAI and Ollama adapters"`

---

### Task 8: Runtime crate — BudgetManager and TracingSink

**Crate**: `disyn-runtime`
**File(s)**: `crates/disyn-runtime/src/budget.rs`,
`crates/disyn-runtime/src/telemetry.rs`,
`crates/disyn-runtime/src/executor.rs`,
`crates/disyn-runtime/src/lib.rs`
**Run**: `cargo test -p disyn-runtime`

1. Write failing tests for budget:

   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn budget_starts_with_capacity() {
           let b = BudgetManager::new(1000, 1, 3);
           assert!(b.can_afford(&CostEstimate {
               input_tokens: 500,
               output_tokens: 200,
           }));
       }

       #[test]
       fn budget_rejects_when_exceeded() {
           let mut b = BudgetManager::new(100, 1, 3);
           b.record(&ResourceUsage {
               total_tokens: 90,
               wall_time_ms: 100,
           });
           assert!(!b.can_afford(&CostEstimate {
               input_tokens: 50,
               output_tokens: 50,
           }));
       }

       #[test]
       fn repair_attempts_decrement() {
           let mut b = BudgetManager::new(1000, 1, 3);
           assert!(b.can_repair());
           b.record_repair();
           b.record_repair();
           b.record_repair();
           assert!(!b.can_repair());
       }
   }
   ```

2. Implement `BudgetManager`:

   ```rust
   use disyn_core::types::{CostEstimate, ResourceUsage};

   pub struct BudgetManager {
       max_tokens: u64,
       max_neural_replans: u32,
       max_repair_attempts: u32,
       used_tokens: u64,
       neural_replans: u32,
       repair_attempts: u32,
   }

   impl BudgetManager {
       pub fn new(
           max_tokens: u64,
           max_neural_replans: u32,
           max_repair_attempts: u32,
       ) -> Self {
           Self {
               max_tokens,
               max_neural_replans,
               max_repair_attempts,
               used_tokens: 0,
               neural_replans: 0,
               repair_attempts: 0,
           }
       }

       pub fn can_afford(&self, estimate: &CostEstimate) -> bool {
           let needed =
               u64::from(estimate.input_tokens)
               + u64::from(estimate.output_tokens);
           self.used_tokens + needed <= self.max_tokens
       }

       pub fn record(&mut self, usage: &ResourceUsage) {
           self.used_tokens += usage.total_tokens;
       }

       pub fn can_repair(&self) -> bool {
           self.repair_attempts < self.max_repair_attempts
       }

       pub fn record_repair(&mut self) {
           self.repair_attempts += 1;
       }

       pub fn can_replan(&self) -> bool {
           self.neural_replans < self.max_neural_replans
       }

       pub fn record_replan(&mut self) {
           self.neural_replans += 1;
       }
   }
   ```

3. Implement `TracingSink`:

   ```rust
   use disyn_core::ports::{SpanEvent, TelemetrySink};

   pub struct TracingSink;

   impl TracingSink {
       pub fn init() -> Self {
           Self
       }
   }

   impl TelemetrySink for TracingSink {
       fn emit(&self, event: &SpanEvent) {
           tracing::info!(
               trace_id = %event.trace_id,
               kind = ?event.kind,
               duration_ms = event.duration_ms,
               status = ?event.status,
               "span"
           );
       }
   }
   ```

4. Implement `ShellExecutor`:

   ```rust
   use async_trait::async_trait;
   use disyn_core::ports::ActionExecutor;
   use disyn_core::types::*;
   use disyn_core::Result;

   pub struct ShellExecutor;

   impl ShellExecutor {
       pub fn new() -> Self {
           Self
       }
   }

   #[async_trait]
   impl ActionExecutor for ShellExecutor {
       async fn execute(
           &self,
           plan: &ApprovedPlan,
       ) -> Result<ExecutionReport> {
           let mut results = Vec::new();
           for (i, step) in plan.steps.iter().enumerate() {
               let output = tokio::process::Command::new("sh")
                   .arg("-c")
                   .arg(&step.action)
                   .output()
                   .await
                   .map_err(|e| {
                       disyn_core::Error::Execution(e.to_string())
                   })?;
               results.push(StepResult {
                   step_index: i,
                   success: output.status.success(),
                   output: serde_json::json!({
                       "stdout": String::from_utf8_lossy(
                           &output.stdout
                       ),
                       "stderr": String::from_utf8_lossy(
                           &output.stderr
                       ),
                   }),
                   error: if output.status.success() {
                       None
                   } else {
                       Some(format!(
                           "exit code: {}",
                           output.status.code().unwrap_or(-1)
                       ))
                   },
               });
           }
           Ok(ExecutionReport {
               results,
               total_cost: ResourceUsage {
                   total_tokens: 0,
                   wall_time_ms: 0,
               },
           })
       }
   }
   ```

5. Update `lib.rs`:

   ```rust
   pub mod budget;
   pub mod executor;
   pub mod telemetry;

   pub use budget::BudgetManager;
   pub use executor::ShellExecutor;
   pub use telemetry::TracingSink;
   ```

6. Verify:

   ```
   cargo test -p disyn-runtime     → 3 tests pass
   cargo clippy -p disyn-runtime   → zero warnings
   ```

7. Commit: `git commit -m "feat(runtime): add BudgetManager, TracingSink, ShellExecutor"`

---

### Task 9: App crate — Orchestrator and main

**Crate**: `disyn-app`
**File(s)**: `crates/disyn-app/src/orchestrator.rs`,
`crates/disyn-app/src/config.rs`, `crates/disyn-app/src/main.rs`
**Run**: `cargo build -p disyn-app`

1. Write `config.rs`:

   ```rust
   use clap::Parser;

   #[derive(Parser)]
   #[command(name = "disyn", version, about = "Hybrid agent pipeline")]
   pub struct Config {
       /// LLM provider: openai or ollama
       #[arg(long, env = "DISYN_PROVIDER", default_value = "openai")]
       pub provider: String,

       /// OpenAI API key
       #[arg(long, env = "OPENAI_API_KEY", default_value = "")]
       pub api_key: String,

       /// Model name
       #[arg(long, env = "DISYN_MODEL", default_value = "gpt-4o")]
       pub model: String,

       /// Max token budget
       #[arg(long, env = "DISYN_MAX_TOKENS", default_value = "10000")]
       pub max_tokens: u64,
   }
   ```

2. Write `orchestrator.rs`:

   ```rust
   use disyn_core::ports::*;
   use disyn_core::types::*;
   use disyn_core::{Error, Result};
   use disyn_runtime::BudgetManager;

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
       pub async fn run(
           &mut self,
           obs: Observation,
       ) -> Result<ExecutionReport> {
           let facts = self.fact_extractor.extract(&obs).await?;
           let memory_ctx = self.memory.retrieve(&facts).await?;
           let mut draft = self
               .proposal_engine
               .propose(&facts, &memory_ctx)
               .await?;

           let approved = match self.verify_loop(&mut draft) {
               Ok(plan) => plan,
               Err(_) if self.budget.can_replan() => {
                   self.budget.record_replan();
                   let mut redraft = self
                       .proposal_engine
                       .propose(&facts, &memory_ctx)
                       .await?;
                   self.verify_loop(&mut redraft)?
               }
               Err(e) => return Err(e),
           };

           let report = self.executor.execute(&approved).await?;
           self.memory.persist(&report).await?;
           self.budget.record(&report.total_cost);
           Ok(report)
       }

       fn verify_loop(
           &mut self,
           draft: &mut PlanDraft,
       ) -> Result<ApprovedPlan> {
           for _ in 0..3 {
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
               violations: self
                   .verifier
                   .verify(draft)
                   .violations
                   .len(),
           })
       }
   }
   ```

3. Write `main.rs`:

   ```rust
   mod config;
   mod orchestrator;

   use clap::Parser;
   use disyn_core::types::Observation;
   use disyn_memory::InMemoryStore;
   use disyn_neural::openai::OpenAiConfig;
   use disyn_neural::{
       OllamaFactExtractor, OllamaProposalEngine,
       OpenAiFactExtractor, OpenAiProposalEngine,
   };
   use disyn_runtime::{BudgetManager, ShellExecutor, TracingSink};
   use disyn_symbolic::{PatternRepairEngine, RuleSetVerifier};

   use config::Config;
   use orchestrator::Orchestrator;

   #[tokio::main]
   async fn main() -> disyn_core::Result<()> {
       tracing_subscriber::fmt()
           .with_env_filter("info")
           .json()
           .init();

       let cfg = Config::parse();

       let openai_cfg = OpenAiConfig {
           api_key: cfg.api_key.clone(),
           model: cfg.model.clone(),
           base_url: if cfg.provider == "ollama" {
               "http://localhost:11434/v1".into()
           } else {
               "https://api.openai.com/v1".into()
           },
       };

       let (fact_extractor, proposal_engine): (
           Box<dyn disyn_core::ports::FactExtractor>,
           Box<dyn disyn_core::ports::ProposalEngine>,
       ) = if cfg.provider == "ollama" {
           (
               Box::new(OllamaFactExtractor::new(openai_cfg.clone())),
               Box::new(OllamaProposalEngine::new(openai_cfg)),
           )
       } else {
           (
               Box::new(OpenAiFactExtractor::new(openai_cfg.clone())),
               Box::new(OpenAiProposalEngine::new(openai_cfg)),
           )
       };

       let mut orchestrator = Orchestrator {
           fact_extractor,
           proposal_engine,
           verifier: Box::new(RuleSetVerifier::default()),
           repair_engine: Box::new(PatternRepairEngine::default()),
           memory: Box::new(InMemoryStore::new()),
           executor: Box::new(ShellExecutor::new()),
           telemetry: Box::new(TracingSink::init()),
           budget: BudgetManager::new(cfg.max_tokens, 1, 3),
       };

       let input = std::io::read_to_string(std::io::stdin())
           .map_err(|e| disyn_core::Error::Other(e.to_string()))?;
       let obs = Observation {
           source: "stdin".into(),
           payload: serde_json::from_str(&input)
               .unwrap_or(serde_json::json!({ "raw": input })),
           timestamp: chrono::Utc::now(),
       };

       let report = orchestrator.run(obs).await?;
       println!("{}", serde_json::to_string_pretty(&report)
           .map_err(|e| disyn_core::Error::Other(e.to_string()))?);
       Ok(())
   }
   ```

4. Verify:

   ```
   cargo build -p disyn-app        → compiles
   cargo clippy -p disyn-app       → zero warnings
   ```

5. Commit: `git commit -m "feat(app): add Orchestrator with repair gate and CLI"`

---

### Task 10: xtask crate — CI runner

**Crate**: `disyn-xtask`
**File(s)**: `crates/disyn-xtask/src/main.rs`
**Run**: `cargo xtask ci`

1. Implement:

   ```rust
   use std::process::Command;

   fn run(label: &str, cmd: &str, args: &[&str]) {
       println!("--- {label} ---");
       let status = Command::new(cmd)
           .args(args)
           .status()
           .unwrap_or_else(|e| panic!("{label} failed to start: {e}"));
       if !status.success() {
           eprintln!("{label} FAILED");
           std::process::exit(1);
       }
   }

   fn main() {
       let args: Vec<String> = std::env::args().collect();
       let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("ci");

       match cmd {
           "ci" => {
               run("fmt", "cargo", &["fmt", "--all", "--check"]);
               run(
                   "clippy",
                   "cargo",
                   &[
                       "clippy", "--workspace", "--all-targets",
                       "--", "-D", "warnings",
                   ],
               );
               run("test", "cargo", &["test", "--workspace"]);
               run("build", "cargo", &["build", "--workspace"]);
               println!("--- all gates passed ---");
           }
           other => {
               eprintln!("unknown command: {other}");
               eprintln!("usage: cargo xtask ci");
               std::process::exit(1);
           }
       }
   }
   ```

2. Verify: `cargo xtask ci` — all gates pass.

3. Commit: `git commit -m "feat(xtask): add cargo xtask ci runner"`

---

### Task 11: Root files and git init

**Crate**: (root)
**File(s)**: `README.md`, `LICENSE-MIT`, `LICENSE-APACHE`, `.gitignore`
**Run**: `cargo xtask ci`

1. Write `.gitignore`:

   ```
   /target
   .env
   ```

2. Write `README.md` with project name, one-line description, crate
   table, build/test instructions.

3. Write `LICENSE-MIT` and `LICENSE-APACHE` (full text, dual license).

4. `git init && git add -A && git commit -m "chore: initial commit"`

5. Create GitHub repo:
   `gh repo create 89jobrien/disyn --public --description "Hybrid symbolic+neural agent pipeline in Rust"`

6. Push:
   `git remote add origin https://github.com/89jobrien/disyn.git && git push -u origin main`
