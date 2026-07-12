# Changelog

## Unreleased

### Features

- **Fail-fast API key validation** — `Config::validate()` errors at startup when `provider=openai`
  and `OPENAI_API_KEY` is empty, rather than propagating an inference error mid-pipeline.
- **Violation messages in errors** — `Error::Verification` now carries `messages: Vec<String>` with
  the specific rule failure text; callers no longer receive only a violation count.
- **Per-step wall-time tracking** — `ShellExecutor` accumulates step durations into
  `ExecutionReport.total_cost.wall_time_ms`.
- **Per-step shell timeout** — `ShellExecutor` enforces a configurable timeout per shell step
  (default 30s; override via `DISYN_STEP_TIMEOUT_SECS`). Hangs now error cleanly.
- **Semantic memory retrieval** — `InMemoryStore::retrieve` scores stored episodes by keyword
  overlap against query entities and returns the top-5 as `WeightedPassage` entries instead of
  always returning an empty context.
- **Memory eviction** — `InMemoryStore` is bounded to 256 episodes; oldest episodes are evicted
  when capacity is exceeded (LRU via `VecDeque`).
- **Compound `ClauseCombinator` handling** — `NoOpFormalVerifier` now evaluates `Not` (negation),
  `Or` (any-in-group passes), and `And`/`Leaf` (all-must-hold) clauses instead of silently
  treating all as satisfied.
- **`cargo deny` in CI** — dependency audit runs on every push and pull request.

### Fixes

- **Ollama/OpenAI error on malformed response** — missing or non-JSON content fields now return
  `Error::Inference` instead of silently producing empty `Facts` or `PlanDraft`.

### Refactoring

- **`OrchestratorPorts` struct** — replaced 8-argument `Orchestrator::new` with a named-field
  struct, eliminating the `clippy::too_many_arguments` suppression.
- **`Orchestrator::emit` helper** — extracted repeated `SpanEvent` construction into a single
  private method, removing three identical boilerplate sites.
- **Shared neural helpers** — extracted `memory_section`, `extract_content`, and
  `parse_plan_draft` into `disyn-neural::shared`, eliminating verbatim duplication between
  the OpenAI and Ollama proposal adapters.
- **Named constants** — replaced magic numbers (`0.0`, `30`, `50`) with named constants across
  `disyn-memory`, `disyn-runtime`, and the pipeline example.
- **`verify_execution_report` decomposition** — split a cyclomatic-12 function into
  `verify_step`, `verify_cost`, and `expected_step_output` helpers in the pipeline example.
- **`#[must_use]` on constructors** — annotated `new()` on `RuleSetVerifier`, `BudgetManager`,
  `ShellExecutor`, `OpenAiFactExtractor`, `OpenAiProposalEngine`, `OllamaFactExtractor`, and
  `OllamaProposalEngine`.

### Quality

- rustqual score: 86.3% → 99.2% (12 findings → 1 residual BP-009 in example fixture code)

## v0.1.0

Initial release.

- Symbolic verification pipeline with 10-layer verification taxonomy
  and FormalVerifier trait
- Frontier-of-Thought (FoT) stub types for structured reasoning traces
- CatRAG graph types and GraphStore trait for graph-based memory
- Per-class budget tracking in the runtime
- Idempotency keys on PlannedStep and StepResult
- Telemetry spans wired into the orchestrator pipeline
- OpenAI and Ollama provider support
