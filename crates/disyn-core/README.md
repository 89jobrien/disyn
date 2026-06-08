# disyn-core

[![crates.io](https://img.shields.io/crates/v/disyn-core.svg)](https://crates.io/crates/disyn-core)

Domain types, port traits, and error types for the
[disyn](https://github.com/89jobrien/disyn) agent pipeline.

## Key types

- `Observation` — raw input entering the pipeline
- `Facts` — extracted structured data
- `PlanDraft` / `PlannedStep` — proposed actions with cost estimates
- `VerificationReport` — symbolic verification output
- `ApprovedPlan` — verified plan ready for execution
- `ExecutionReport` — step results and resource usage

## Port traits

All cross-crate boundaries are defined here as async traits:

- `FactExtractor` — observation to facts
- `ProposalEngine` — facts + memory to plan draft
- `Verifier` — plan validation
- `RepairEngine` — fix verification violations
- `MemoryStore` — persist and retrieve context
- `ActionExecutor` — execute approved steps
- `TelemetrySink` — emit pipeline telemetry

## License

MIT OR Apache-2.0
