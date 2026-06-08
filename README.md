# disyn

[![crates.io](https://img.shields.io/crates/v/disyn-core.svg)](https://crates.io/crates/disyn-core)
[![license](https://img.shields.io/crates/l/disyn-core.svg)](LICENSE-MIT)

Hybrid symbolic+neural agent pipeline in Rust. Transforms raw
observations into verified, executed actions through a typed 7-stage
pipeline where neural proposals are validated and repaired by symbolic
logic before execution.

## Pipeline

```
Observation -> FactExtractor -> MemoryStore::retrieve -> ProposalEngine
  -> Verifier -> [RepairEngine loop, max 3] -> ApprovedPlan
  -> ActionExecutor -> ExecutionReport -> MemoryStore::persist
```

If verification fails after 3 repair attempts and budget allows, the
orchestrator replans (re-proposes from scratch) once before erroring.

## Crates

| Crate | Description |
| --- | --- |
| [`disyn-core`](crates/disyn-core) | Domain types, port traits, error types |
| [`disyn-symbolic`](crates/disyn-symbolic) | Rule engine, verifier, repair engine, 10-layer verification taxonomy |
| [`disyn-neural`](crates/disyn-neural) | LLM adapters (OpenAI, Ollama) |
| [`disyn-memory`](crates/disyn-memory) | State persistence, in-memory store, CatRAG graph types |
| [`disyn-runtime`](crates/disyn-runtime) | Budget manager (per-class tracking), telemetry, shell executor |
| [`disyn-app`](crates/disyn-app) | Composition root, orchestrator, CLI |
| `disyn-xtask` | CI automation (`cargo xtask ci`) |

### Dependency graph

```
disyn-core
  disyn-symbolic
  disyn-neural
  disyn-memory
  disyn-runtime
    disyn-app (depends on all above)
```

## Architecture

Hexagonal design — all cross-crate boundaries use trait ports defined
in `disyn-core::ports`: `FactExtractor`, `ProposalEngine`, `Verifier`,
`RepairEngine`, `MemoryStore`, `ActionExecutor`, `TelemetrySink`. The
`Orchestrator` in `disyn-app` holds `Box<dyn Port>` for each.

## Build

```sh
cargo xtask ci          # fmt + clippy + test + build
cargo build --workspace
cargo test --workspace
```

## Configuration

| Env var | Default | Description |
| --- | --- | --- |
| `DISYN_PROVIDER` | `openai` | LLM provider (`openai` or `ollama`) |
| `OPENAI_API_KEY` | (empty) | OpenAI API key |
| `DISYN_MODEL` | `gpt-4o` | Model name |
| `DISYN_MAX_TOKENS` | `10000` | Token budget |

## License

MIT OR Apache-2.0
