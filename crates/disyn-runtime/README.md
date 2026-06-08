# disyn-runtime

[![crates.io](https://img.shields.io/crates/v/disyn-runtime.svg)](https://crates.io/crates/disyn-runtime)

Budget manager, telemetry, and shell executor for the
[disyn](https://github.com/89jobrien/disyn) agent pipeline.

## Features

- `BudgetManager` — token budget enforcement with per-class tracking
  (neural, repair, replan)
- `ShellExecutor` — `ActionExecutor` implementation via subprocess
- Telemetry span integration

## License

MIT OR Apache-2.0
