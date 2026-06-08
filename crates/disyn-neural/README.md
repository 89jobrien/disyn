# disyn-neural

[![crates.io](https://img.shields.io/crates/v/disyn-neural.svg)](https://crates.io/crates/disyn-neural)

Neural proposal engine and fact extractor for the
[disyn](https://github.com/89jobrien/disyn) agent pipeline.

## Providers

- **OpenAI** — `OpenAiExtractor` and `OpenAiProposer` via the
  chat completions API
- **Ollama** — local model support (same trait interface)

## Configuration

| Env var | Default | Description |
| --- | --- | --- |
| `DISYN_PROVIDER` | `openai` | Provider selection |
| `OPENAI_API_KEY` | (empty) | OpenAI API key |
| `DISYN_MODEL` | `gpt-4o` | Model name |

## License

MIT OR Apache-2.0
