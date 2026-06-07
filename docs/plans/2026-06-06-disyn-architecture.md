# disyn Architecture Design

Date: 2026-06-06

## Goal

Build a domain-agnostic hybrid agent system in Rust that combines stochastic
neural perception with deterministic symbolic verification. The system
transforms raw observations into verified, executed actions through a typed
7-stage pipeline. Neural proposals are validated and repaired by symbolic
logic before execution, reserving expensive LLM compute for novel reasoning.

Providers: OpenAI (primary), Ollama (local validation/testing).

## Architecture

### Crate Topology (7 crates)

```
disyn/
├── Cargo.toml                # virtual manifest, resolver 3
├── rust-toolchain.toml       # pin 1.88, clippy + rustfmt
├── .cargo/config.toml        # shared compiler flags
├── crates/
│   ├── disyn-core/           # domain types, port traits (zero workspace deps)
│   ├── disyn-symbolic/       # rule engine, verifier, repair (deterministic)
│   ├── disyn-neural/         # LLM adapters: OpenAI, Ollama (stochastic)
│   ├── disyn-memory/         # state persistence, RAG store
│   ├── disyn-runtime/        # async executor, budget, telemetry
│   ├── disyn-app/            # composition root, DI wiring, orchestrator
│   └── disyn-xtask/          # cargo xtask ci automation (dev only)
```

### Dependency Graph

```
disyn-app → disyn-symbolic → disyn-core
          → disyn-neural   → disyn-core
          → disyn-memory   → disyn-core
          → disyn-runtime  → disyn-core
          → disyn-core

disyn-core → (nothing in workspace)
disyn-xtask → (nothing in workspace, dev-only)
```

No cross-adapter dependencies. symbolic/neural/memory/runtime never depend
on each other.

### Port Traits (disyn-core)

Four categories, all object-safe for runtime pluggability:

**Neural ports (async):**

- `FactExtractor` -- extract typed facts from raw observation
- `ProposalEngine` -- generate PlanDraft from facts + memory context

**Symbolic ports (sync, deterministic):**

- `Verifier` -- validate PlanDraft against rules, produce VerificationReport
- `RepairEngine` -- deterministically fix violations, return corrected PlanDraft

**Memory ports (async):**

- `MemoryStore` -- retrieve context for facts, persist execution outcomes

**Runtime ports (async):**

- `ActionExecutor` -- dispatch ApprovedPlan, return ExecutionReport

**Cross-cutting:**

- `TelemetrySink` -- emit structured span events
- `Budget` (concrete struct, not trait) -- token/cost accounting

### Pipeline DTOs (disyn-core::types)

All derive `Debug, Clone, Serialize, Deserialize`. All cross-crate
communication uses these typed structs.

| Stage | DTO                  | Description                               |
| ----- | -------------------- | ----------------------------------------- |
| 1     | `Observation`        | Raw input (source, payload, timestamp)    |
| 2     | `Facts`              | Extracted entities, relations, confidence |
| 3     | `MemoryContext`      | Retrieved episodes + summary              |
| 4     | `PlanDraft`          | Proposed steps + rationale                |
| 5     | `VerificationReport` | Pass/fail + violations list               |
| 6     | `ApprovedPlan`       | Verified steps + report (audit trail)     |
| 7     | `ExecutionReport`    | Step results + total resource usage       |

Supporting types: `PlannedStep`, `Episode`, `Violation`, `Severity`,
`CostEstimate`, `ResourceUsage`, `StepResult`.

### Repair Gate Flow

```
PlanDraft → Verifier::verify()
  → passed → ApprovedPlan
  → failed → RepairEngine::repair() (up to 3 attempts, re-verify each)
    → repair exhausted → ProposalEngine::propose() (1 neural replan)
      → still fails → abort with VerificationReport
```

Symbolic repair is cheaper, safer, and testable. Neural replan is the
last resort, capped at 1 attempt to prevent runaway costs.

### Observability

Structured spans via `tracing`, correlated by `trace_id`:

| Span                | Boundary                                |
| ------------------- | --------------------------------------- |
| `proposal.generate` | Neural: fact extraction + plan drafting |
| `verifier.check`    | Symbolic: rule validation               |
| `repair.apply`      | Symbolic: deterministic correction      |
| `executor.dispatch` | Runtime: action execution               |

`TelemetrySink` trait in core; `TracingSink` adapter in runtime wraps
`tracing::instrument`. Bridge to OpenTelemetry via tracing subscribers.

### Budget Accounting

`BudgetManager` in disyn-runtime (concrete struct):

- `max_tokens`, `max_neural_replans` (1), `max_repair_attempts` (3)
- Orchestrator checks `can_repair()` / `can_replan()` before each attempt
- `record()` called after execution with actual usage

### Composition Root (disyn-app)

`Orchestrator` struct holds `Box<dyn Trait>` for all ports. Single `run()`
method drives the full pipeline. `main.rs` wires concrete adapters based
on config/env:

- OpenAI adapters (default) or Ollama adapters (via config switch)
- `RuleSetVerifier` + `PatternRepairEngine` (placeholder symbolic impls)
- `InMemoryStore` (initial memory impl)
- `ShellExecutor` (action dispatch)
- `TracingSink` (telemetry)

Config loaded from env vars. Provider swap requires no code changes --
only config.

## Tech Decisions

| Decision                                        | Rationale                                                |
| ----------------------------------------------- | -------------------------------------------------------- |
| `Box<dyn Trait>` over generics for orchestrator | Runtime pluggability, avoids generic parameter explosion |
| Sync symbolic traits                            | Deterministic, no I/O -- simpler, testable               |
| Async neural/memory/executor traits             | I/O bound, needs tokio                                   |
| `serde_json::Value` for extensible params       | Domain-agnostic; no fixed schema for step parameters     |
| Concrete `BudgetManager` (not trait)            | Only one way to count tokens; no need for abstraction    |
| Resolver 3                                      | Edition 2024 default, prevents feature leakage           |
| `thiserror` for core error type                 | Ergonomic, derives std::error::Error                     |
| `uuid` for trace correlation                    | Standard, interops with OpenTelemetry                    |

## Out of Scope

- Domain-specific rules/verifiers (placeholder only; filled per use case)
- Streaming/SSE from LLM providers (batch inference first)
- Persistent storage backends (SQLite, Postgres) -- start with in-memory
- Desktop/TUI -- CLI-only initially
- Multi-agent coordination -- single agent pipeline only
- Authentication/authorization for API access
- Prompt engineering specifics (adapter-internal concern)
