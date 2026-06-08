# Usage

## Build

```sh
cargo build --workspace
cargo xtask ci              # fmt-check + clippy (-D warnings) + test + build
```

## Test

```sh
cargo test --workspace
cargo test -p disyn-core                          # single crate
cargo test -p disyn-core -- observation_round_trips  # single test
```

## Configuration

Provider selection and budget are controlled via environment variables:

| Env var | Default | Description |
|---|---|---|
| `DISYN_PROVIDER` | `openai` | LLM provider (`openai` or `ollama`) |
| `OPENAI_API_KEY` | (required for openai) | OpenAI API key |
| `DISYN_MODEL` | `gpt-4o` | Model name |
| `DISYN_MAX_TOKENS` | `10000` | Token budget |

## Quality

```sh
rustqual .    # score code quality (IOSP, complexity, DRY, SRP, coupling)
```

Configured via `rustqual.toml` in the workspace root.
