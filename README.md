# disyn

Hybrid symbolic+neural agent pipeline in Rust. Transforms raw observations
into verified, executed actions through a typed 7-stage pipeline where
neural proposals are validated and repaired by symbolic logic before
execution.

## Crates

| Crate            | Description                               |
| ---------------- | ----------------------------------------- |
| `disyn-core`     | Domain types, port traits, error types    |
| `disyn-symbolic` | Rule engine, verifier, repair engine      |
| `disyn-neural`   | LLM adapters (OpenAI, Ollama)             |
| `disyn-memory`   | State persistence, in-memory store        |
| `disyn-runtime`  | Budget manager, telemetry, shell executor |
| `disyn-app`      | Composition root, orchestrator, CLI       |
| `disyn-xtask`    | CI automation (`cargo xtask ci`)          |

## Pipeline

```
Observation -> Facts -> MemoryContext -> PlanDraft
  -> Verify -> [Repair loop] -> ApprovedPlan -> Execute -> ExecutionReport
```

## Build

```sh
cargo xtask ci          # fmt + clippy + test + build
cargo build --workspace
cargo test --workspace
```

## Configuration

| Env var            | Default  | Description                         |
| ------------------ | -------- | ----------------------------------- |
| `DISYN_PROVIDER`   | `openai` | LLM provider (`openai` or `ollama`) |
| `OPENAI_API_KEY`   | (empty)  | OpenAI API key                      |
| `DISYN_MODEL`      | `gpt-4o` | Model name                          |
| `DISYN_MAX_TOKENS` | `10000`  | Token budget                        |

## License

MIT OR Apache-2.0
