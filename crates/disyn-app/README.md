# disyn-app

[![crates.io](https://img.shields.io/crates/v/disyn-app.svg)](https://crates.io/crates/disyn-app)

Orchestrator and CLI for the
[disyn](https://github.com/89jobrien/disyn) hybrid symbolic-neural
agent pipeline.

## Overview

This is the composition root — it wires all port implementations
together and runs the full pipeline:

```
Observation -> FactExtractor -> MemoryStore::retrieve -> ProposalEngine
  -> Verifier -> [RepairEngine loop, max 3] -> ApprovedPlan
  -> ActionExecutor -> ExecutionReport -> MemoryStore::persist
```

## Dependencies

Depends on all other disyn crates: `disyn-core`, `disyn-symbolic`,
`disyn-neural`, `disyn-memory`, `disyn-runtime`.

## License

MIT OR Apache-2.0
