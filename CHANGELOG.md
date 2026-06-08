# Changelog

## v0.1.0

### What's New

disyn is a hybrid symbolic+neural agent pipeline for Rust. Neural LLM
proposals are validated and repaired by a symbolic rule engine before
execution.

- **Symbolic verification pipeline** -- a 10-layer verification taxonomy
  and formal verifier trait let you define rule-based checks that gate
  every LLM-generated plan before it runs.
- **Frontier-of-Thought (FoT) reasoning** -- stub types for structured
  reasoning traces, enabling symbolic inspection of how proposals were
  derived.
- **Graph-based memory (CatRAG)** -- a category-theoretic
  retrieval-augmented generation layer stores and retrieves context as a
  typed knowledge graph.
- **Per-class budget tracking** -- set token and cost budgets by resource
  class (e.g. planning vs. execution) to control spend granularly.
- **Idempotent plan steps** -- every planned step carries an idempotency
  key, preventing duplicate execution on retries.
- **Telemetry-instrumented orchestrator** -- the full pipeline
  (observe -> extract -> propose -> verify -> repair -> execute) is wired
  with tracing spans out of the box.

### Architecture

Six crates, all published to crates.io:

| Crate | Purpose |
|-------|---------|
| `disyn-core` | Domain types, port traits, errors |
| `disyn-symbolic` | Verifier and repair engine |
| `disyn-neural` | LLM proposal and fact extraction (OpenAI/Ollama) |
| `disyn-memory` | In-memory context store |
| `disyn-runtime` | Budget manager, telemetry, shell executor |
| `disyn-app` | Orchestrator composition root and CLI |

### Configuration

Set `DISYN_PROVIDER` (openai or ollama), `OPENAI_API_KEY`,
`DISYN_MODEL` (default gpt-4o), and `DISYN_MAX_TOKENS` (default 10000)
to get started.

### License

MIT OR Apache-2.0
