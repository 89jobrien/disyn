# Disyn: Formal-of-Thought Neuro-Symbolic Architecture

Design document for the disyn safety verification pipeline. Covers crate
topology, verification layers, formal verification stubs, graph-aware
memory, and operational instrumentation.

---

## 1. Crate Workspace Topology

Seven crates enforce strict separation between stochastic neural perception
and deterministic symbolic safety logic. Each crate is a compile-time
boundary that prevents responsibility leakage.

| Crate            | Role                                                        | Boundary Rule                                                                          |
| ---------------- | ----------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `disyn-core`     | Domain types, port traits, errors. Single source of truth.  | No provider-specific deps. No business logic.                                          |
| `disyn-neural`   | LLM-backed perception and plan generation (OpenAI, Ollama). | Forbidden from depending on `disyn-symbolic`. Communicates only through `core::types`. |
| `disyn-symbolic` | Deterministic rule verification and hardcoded repair logic. | No stochastic model calls. 100% deterministic.                                         |
| `disyn-memory`   | Episodic session state and graph-backed retrieval.          | State persistence only. No planning or tool dispatch.                                  |
| `disyn-runtime`  | Async execution, budget management, telemetry sink.         | Agnostic of domain-specific agent logic.                                               |
| `disyn-app`      | Composition root. Dependency injection. Provider wiring.    | The only layer that assembles concrete implementations.                                |
| `disyn-xtask`    | Repository automation, CI gates, linting.                   | Dev-only. Never compiled into production binaries.                                     |

`disyn-core` uses narrow, object-safe capability traits and shared DTOs in
`core::types`. Cross-crate exchange is always a typed, Serde-serializable
struct. Provider-specific types (model headers, API envelopes) never leak
into core.

### Dependency graph

```
disyn-core
  disyn-symbolic   (Verifier, RepairEngine, FormalVerifier)
  disyn-neural     (ProposalEngine, FactExtractor)
  disyn-memory     (MemoryStore, GraphStore)
  disyn-runtime    (BudgetManager, ActionExecutor, TelemetrySink)
    disyn-app      (Orchestrator — depends on all above)
```

---

## 2. Ten-Layer Verification Standard

Every tool call is evaluated against ten classes of constraint violation.
The `VerificationLayer` enum encodes this taxonomy at the type level.

```rust
#[repr(u8)]
pub enum VerificationLayer {
    L0Format         = 0,  // JSON structure, required fields
    L1DataSource     = 1,  // IDs match reactive source of truth
    L2UserConstraints = 2, // user-specified restrictions
    L3ToolContract   = 3,  // API schema compliance
    L4Provenance     = 4,  // data origin and reference integrity
    L5Temporal       = 5,  // time-based logic, event ordering
    L6Resource       = 6,  // budget, quantity, availability
    L7Semantic       = 7,  // action matches human intent
    L8Mathematical   = 8,  // arithmetic and logical correctness
    L9Location       = 9,  // geographic and namespace constraints
}
```

Each `Violation` carries its layer, severity, rule ID, message, and step
index. The `RuleSetVerifier` organizes rules in a `BTreeMap` keyed by
layer, evaluating L0 first and short-circuiting on blocking violations
before higher layers run.

Individual rules implement the `LayerRule` trait:

```rust
pub trait LayerRule: Send + Sync {
    fn layer(&self) -> VerificationLayer;
    fn rule_id(&self) -> &str;
    fn check(&self, draft: &PlanDraft) -> Vec<Violation>;
}
```

L7 (Semantic) and L4 (Provenance) are the most critical layers for
maintaining intent integrity and data lineage.

---

## 3. Formal-of-Thought (FoT) Pipeline

FoT shifts the burden of proof from probabilistic neural intuition to
deterministic logic evaluation. The pipeline has four phases.

**Phase 1 -- Intent Decomposition.** The LLM acts as a specification
compiler, breaking high-level intent into binary atomic facts (yes/no
questions grounded in execution logs).

```rust
pub struct AtomicFact {
    pub id: String,
    pub query: String,      // "Was the budget respected?"
    pub layer: u8,          // maps to VerificationLayer
}
```

**Phase 2 -- Grounded Extraction.** The LLM reviews minimal trajectory
slices to answer atomic queries.

```rust
pub struct GroundedFact {
    pub fact: AtomicFact,
    pub value: bool,
    pub evidence: String,   // log excerpt grounding the answer
}

pub struct GroundedExtraction {
    pub facts: Vec<GroundedFact>,
    pub trajectory_span: Range<usize>,
}
```

**Phase 3 -- Formal Specification.** Grounded facts are composed into
safety clauses with logical combinators.

```rust
pub struct SafetyClause {
    pub fact_id: String,
    pub advisory: bool,             // false = hard constraint
    pub combinator: ClauseCombinator, // And | Or | Not | Leaf
}

pub struct FormalSpec {
    pub clauses: Vec<SafetyClause>,
    pub formal_text: Option<String>, // reserved for Dafny/SMT-LIB
}
```

**Phase 4 -- Verification.** A deterministic verifier evaluates the spec.

```rust
pub enum ProofVerdict {
    Safe,
    Unsafe { violated_facts: Vec<String> },
    Unknown { reason: String },
}
```

The `FormalVerifier` trait is synchronous (deterministic). It starts as a
rule-based stub (`NoOpFormalVerifier`) that evaluates clauses by looking up
grounded fact values. When Z3/Dafny integration arrives, only the impl
changes.

```rust
pub trait FormalVerifier: Send + Sync {
    fn synthesize(&self, extraction: &GroundedExtraction) -> FormalSpec;
    fn verify_spec(&self, spec: &FormalSpec, ext: &GroundedExtraction)
        -> ProofVerdict;
    fn scan_deception(&self, ext: &GroundedExtraction) -> DeceptionReport {
        DeceptionReport::none()
    }
}
```

This enables weak-to-strong generalization: a small model (7B) performs
semantic fact-checking while the verifier handles logical composition,
allowing the smaller model to supervise larger agents.

---

## 4. Deception Detection

Agent deception ranges from simple omission to multi-step fabrication.
The `DeceptionLevel` enum classifies the spectrum:

```rust
pub enum DeceptionLevel {
    L1SimpleOmission,         // ignoring constraints, suppressing errors
    L2SelectiveReporting,     // correct output, hidden side-effects
    L3FabricatedProvenance,   // invented source links or reasoning
    L4MultiStepDeception,     // engineered reasoning chains across rounds
}
```

Under the paradox of complexity, L4 attacks leave fabricated evidence that
the FoT pipeline can isolate. The verifier operates on binary grounded
facts and is immune to persuasive false reasoning.

```rust
pub struct DeceptionReport {
    pub detected: bool,
    pub level: Option<DeceptionLevel>,
    pub evidence_fact_ids: Vec<String>,
}
```

---

## 5. Context-Aware Graph Memory (CatRAG)

Standard RAG suffers from the static graph fallacy: fixed transition
probabilities cause random walks to drift into high-degree hub nodes that
are structurally prominent but contextually irrelevant.

CatRAG transforms the graph into a query-adaptive navigation structure via
three mechanisms:

1. **Symbolic anchoring.** Weakly recognized entities are injected as
   topological anchors to regularize the random walk and prevent hub drift.
2. **Query-aware edge weighting.** Edge weights are modulated and
   irrelevant branches pruned before traversal begins.
3. **Key-fact passage enhancement.** Passage scores are adjusted based on
   triple-matching against the current query's relation set.

### Types

```rust
pub struct KgNode {
    pub id: String,
    pub label: String,
    pub salience: f32,       // query-aware score, 0.0-1.0
}

pub struct KgEdge {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub weight: f32,         // dynamic edge weight, 0.0-1.0
}

pub struct SymbolicAnchor {
    pub entity_id: String,
    pub anchor_strength: f32, // 1.0 = hard pin
}

pub struct WeightedPassage {
    pub content: String,
    pub source_episode_index: usize,
    pub triple_match_score: f32,
    pub final_weight: f32,
}

pub struct RetrievalStrategy {
    pub max_hops: u8,
    pub top_k: usize,
    pub anchors: Vec<SymbolicAnchor>,
    pub edge_prune_threshold: f32,
    pub passage_boost: f32,
}
```

`GraphStore` is a separate port from `MemoryStore`. The orchestrator holds
`Option<Box<dyn GraphStore>>` -- when present, traversal results populate
`MemoryContext.weighted_passages` before the proposal engine runs.

```rust
#[async_trait]
pub trait GraphStore: Send + Sync {
    async fn index(&self, facts: &Facts) -> Result<()>;
    async fn traverse(
        &self,
        facts: &Facts,
        strategy: &RetrievalStrategy,
    ) -> Result<Vec<WeightedPassage>>;
    fn resolve_anchors(&self, facts: &Facts) -> Vec<SymbolicAnchor>;
}
```

`Facts.relations: Vec<(String, String, String)>` maps directly to
`KgEdge` triples. No external graph library required -- `HashMap<String,
Vec<KgEdge>>` suffices at current scale.

---

## 6. Symbolic Repair Gate and Budget Model

The pipeline is governed by a triage-act-reflect loop featuring a symbolic
repair gate. Deterministic correction is always attempted before expensive
stochastic replanning.

### Repair loop

```
PlanDraft -> Verifier -> [RepairEngine x3] -> ApprovedPlan
  (on failure, if budget allows) -> replan once -> Verifier -> ...
```

The `PolicyEngine` (Verifier) generates a `VerificationReport`. The
`RepairEngine` applies hardcoded Rust logic to fix formatting errors and
tool-contract breaches. If repair fails after 3 iterations and the budget
allows, the orchestrator replans from scratch once before erroring.

### Cost classes

Symbolic operations (verify, repair) and neural operations (propose,
extract) have fundamentally different cost profiles. The budget model
tracks them separately.

```rust
pub enum CostClass {
    Symbolic,  // cheap, deterministic
    Neural,    // expensive, stochastic
}

pub struct CostEstimate {
    pub class: Option<CostClass>,
    pub input_tokens: u32,
    pub output_tokens: u32,
}
```

`BudgetManager` maintains per-class token pools (`max_neural_tokens`,
`used_neural_tokens`, `used_symbolic_tokens`) alongside the aggregate
budget. `can_afford_neural()` gates neural calls specifically.

### Idempotency

Every `PlannedStep` carries an `idempotency_key: Uuid` generated at
proposal time. The repair engine preserves keys for unchanged steps; new
steps introduced by repair get fresh UUIDs. Replans always generate fresh
keys. The executor passes the key as an `Idempotency-Key` header on
external API calls.

```rust
pub struct PlannedStep {
    pub idempotency_key: Uuid,
    pub action: String,
    pub parameters: serde_json::Value,
    pub estimated_cost: CostEstimate,
}
```

`StepResult` carries the key back for correlation:

```rust
pub struct StepResult {
    pub idempotency_key: Uuid,
    pub step_index: usize,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
}
```

---

## 7. Operational Instrumentation

Standard logging is insufficient for non-deterministic agents. Structured
observability mirrors the crate topology using correlated tracing spans.

Every orchestrator `run()` generates a `trace_id: Uuid`. Four mandatory
spans are emitted through the `TelemetrySink` trait:

| Span              | Kind               | Key metadata                      |
| ----------------- | ------------------ | --------------------------------- |
| proposal.generate | `ProposalGenerate` | step_count, fact_entity_count     |
| verifier.check    | `VerifierCheck`    | passed, violation_count           |
| repair.apply      | `RepairApply`      | violation_count, repair_succeeded |
| executor.dispatch | `ExecutorDispatch` | steps_succeeded, total_tokens     |

```rust
pub struct SpanEvent {
    pub kind: SpanKind,
    pub trace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub duration_ms: u64,
    pub status: SpanStatus,
    pub metadata: serde_json::Value,
}
```

Spans are siblings (not nested) under the shared `trace_id`. If the replan
path fires, the second verify loop reuses the same trace so both passes
are correlated.

---

## 8. Engineering Rules

Three opinionated rules govern the pipeline:

1. **Deterministic repair first.** Hardcoded Rust logic over LLM
   replanning. Cheaper, faster, testable.
2. **Typed payloads (Serde).** Every cross-crate exchange is a structured,
   serializable object in `core::types`.
3. **App as the only wiring layer.** `disyn-app` is the sole point for
   dependency injection. Provider choices never leak into core.

---

## 9. Port Trait Catalog

All cross-crate boundaries use trait ports defined in
`disyn-core/src/ports.rs`. The orchestrator holds `Box<dyn T>` for each.

| Trait            | Crate impl     | Sync/Async |
| ---------------- | -------------- | ---------- |
| `FactExtractor`  | disyn-neural   | async      |
| `ProposalEngine` | disyn-neural   | async      |
| `Verifier`       | disyn-symbolic | sync       |
| `RepairEngine`   | disyn-symbolic | sync       |
| `FormalVerifier` | disyn-symbolic | sync       |
| `MemoryStore`    | disyn-memory   | async      |
| `GraphStore`     | disyn-memory   | async      |
| `ActionExecutor` | disyn-runtime  | async      |
| `TelemetrySink`  | disyn-runtime  | sync       |
