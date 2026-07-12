use std::time::Instant;

use async_trait::async_trait;
use disyn_core::Result;
use disyn_core::ports::ActionExecutor;
use disyn_core::types::{ApprovedPlan, ExecutionReport, ResourceUsage, StepResult};

pub struct ShellExecutor {
    step_timeout_secs: u64,
}

impl ShellExecutor {
    #[must_use]
    pub fn new(step_timeout_secs: u64) -> Self {
        Self { step_timeout_secs }
    }
}

const DEFAULT_STEP_TIMEOUT_SECS: u64 = 30;

impl Default for ShellExecutor {
    fn default() -> Self {
        Self::new(DEFAULT_STEP_TIMEOUT_SECS)
    }
}

#[async_trait]
impl ActionExecutor for ShellExecutor {
    async fn execute(&self, plan: &ApprovedPlan) -> Result<ExecutionReport> {
        let mut results = Vec::new();
        let mut total_wall_ms: u64 = 0;

        for (i, step) in plan.steps.iter().enumerate() {
            let t = Instant::now();
            let output = tokio::time::timeout(
                std::time::Duration::from_secs(self.step_timeout_secs),
                tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&step.action)
                    .output(),
            )
            .await
            .map_err(|_| {
                disyn_core::Error::Execution(format!(
                    "step {i} timed out after {}s",
                    self.step_timeout_secs
                ))
            })?
            .map_err(|e| disyn_core::Error::Execution(e.to_string()))?;

            let elapsed_ms = t.elapsed().as_millis() as u64;
            total_wall_ms += elapsed_ms;

            results.push(StepResult {
                idempotency_key: step.idempotency_key,
                step_index: i,
                success: output.status.success(),
                output: serde_json::json!({
                    "stdout": String::from_utf8_lossy(&output.stdout),
                    "stderr": String::from_utf8_lossy(&output.stderr),
                    "wall_time_ms": elapsed_ms,
                }),
                error: if output.status.success() {
                    None
                } else {
                    Some(format!("exit code: {}", output.status.code().unwrap_or(-1)))
                },
            });
        }

        Ok(ExecutionReport {
            results,
            total_cost: ResourceUsage {
                total_tokens: 0,
                symbolic_tokens: 0,
                neural_tokens: 0,
                wall_time_ms: total_wall_ms,
            },
        })
    }
}
