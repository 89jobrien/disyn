# Architecture

Hybrid symbolic+neural agent pipeline. Neural LLM proposals are validated
and repaired by a symbolic rule engine before execution.

## Pipeline Flow

```
Observation -> FactExtractor -> MemoryStore::retrieve -> ProposalEngine
  -> Verifier -> [RepairEngine loop, max 3] -> ApprovedPlan
  -> ActionExecutor -> ExecutionReport -> MemoryStore::persist
```

If verification fails after 3 repair attempts and budget allows, the
orchestrator replans (re-proposes from scratch) once before erroring.

## Crates

| Crate | Purpose |
|---|---|
| `disyn-core` | Domain types, port traits, error types |
| `disyn-symbolic` | Rule engine, verifier, repair engine, 10-layer verification taxonomy |
| `disyn-neural` | LLM adapters (OpenAI, Ollama) |
| `disyn-memory` | State persistence, in-memory store, CatRAG graph types |
| `disyn-runtime` | Budget manager (per-class tracking), telemetry, shell executor |
| `disyn-app` | Composition root, orchestrator, CLI |
| `disyn-xtask` | CI automation (`cargo xtask ci`) |

## Dependency Graph

```
disyn-core          (domain types, port traits, errors -- no dependencies)
  disyn-symbolic    (Verifier + RepairEngine impls)
  disyn-neural      (ProposalEngine + FactExtractor impls via OpenAI/Ollama)
  disyn-memory      (MemoryStore impl -- in-memory)
  disyn-runtime     (BudgetManager, telemetry, shell executor)
    disyn-app       (Orchestrator composition root, CLI -- depends on all above)
```

## Hexagonal Ports

All cross-crate boundaries use trait ports defined in
`disyn-core::ports`:

- `FactExtractor` -- extracts structured facts from raw observations
- `ProposalEngine` -- generates plan drafts from facts and memory context
- `Verifier` -- validates plan drafts against symbolic rules
- `RepairEngine` -- attempts to fix verification violations
- `MemoryStore` -- persists and retrieves context
- `ActionExecutor` -- executes approved plan steps
- `TelemetrySink` -- receives tracing spans and metrics

The `Orchestrator` in `disyn-app` holds `Box<dyn Port>` for each,
allowing any implementation to be swapped at construction time.

## Key Types

Defined in `disyn-core::types`:

```
Observation -> Facts -> MemoryContext -> PlanDraft (steps + rationale)
  -> VerificationReport (violations) -> ApprovedPlan
  -> ExecutionReport (step results + resource usage)
```

Each `PlannedStep` carries a `CostEstimate` (input/output tokens) and
an idempotency key to prevent duplicate execution on retries.
