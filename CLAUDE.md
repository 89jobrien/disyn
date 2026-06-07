# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Build & Test

```sh
cargo xtask ci              # fmt-check + clippy (-D warnings) + test + build
cargo build --workspace
cargo test --workspace
cargo test -p disyn-core     # single crate
cargo test -p disyn-core -- observation_round_trips  # single test
```

Rust edition 2024, toolchain 1.88. `set_var`/`remove_var` require `unsafe {}`.

## Architecture

Hybrid symbolic+neural agent pipeline. Neural LLM proposals are validated and
repaired by a symbolic rule engine before execution.

### Pipeline flow

```
Observation -> FactExtractor -> MemoryStore::retrieve -> ProposalEngine
  -> Verifier -> [RepairEngine loop, max 3] -> ApprovedPlan
  -> ActionExecutor -> ExecutionReport -> MemoryStore::persist
```

If verification fails after 3 repair attempts and budget allows, the
orchestrator replans (re-proposes from scratch) once before erroring.

### Crate dependency graph

```
disyn-core          (domain types, port traits, errors — no dependencies)
  disyn-symbolic    (Verifier + RepairEngine impls)
  disyn-neural      (ProposalEngine + FactExtractor impls via OpenAI/Ollama)
  disyn-memory      (MemoryStore impl — in-memory)
  disyn-runtime     (BudgetManager, telemetry, shell executor)
    disyn-app       (Orchestrator composition root, CLI — depends on all above)
```

### Hexagonal ports (disyn-core/src/ports.rs)

All cross-crate boundaries use trait ports: `FactExtractor`, `ProposalEngine`,
`Verifier`, `RepairEngine`, `MemoryStore`, `ActionExecutor`, `TelemetrySink`.
The `Orchestrator` in disyn-app holds `Box<dyn Port>` for each.

### Key types (disyn-core/src/types.rs)

`Observation` -> `Facts` -> `MemoryContext` -> `PlanDraft` (steps +
rationale) -> `VerificationReport` (violations) -> `ApprovedPlan` ->
`ExecutionReport` (step results + resource usage). Each `PlannedStep`
carries a `CostEstimate` (input/output tokens).

## Configuration

Provider selection and budget via env vars: `DISYN_PROVIDER` (openai|ollama),
`OPENAI_API_KEY`, `DISYN_MODEL` (default gpt-4o), `DISYN_MAX_TOKENS`
(default 10000).

## Quality

`rustqual.toml` configures the rustqual analyzer. Run with `rustqual .` to
score code quality across IOSP, complexity, DRY, SRP, coupling, test quality,
and architecture dimensions.

## License

MIT OR Apache-2.0
