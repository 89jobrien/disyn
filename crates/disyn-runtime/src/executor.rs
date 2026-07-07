use async_trait::async_trait;
use disyn_core::Result;
use disyn_core::ports::ActionExecutor;
use disyn_core::types::{ApprovedPlan, ExecutionReport, ResourceUsage, StepResult};

pub struct ShellExecutor;

impl ShellExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ShellExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ActionExecutor for ShellExecutor {
    async fn execute(&self, plan: &ApprovedPlan) -> Result<ExecutionReport> {
        // TODO: Track wall time and token usage per step and populate total_cost in ExecutionReport.
        let mut results = Vec::new();
        for (i, step) in plan.steps.iter().enumerate() {
            // TODO: Apply a per-step timeout to prevent shell commands from hanging indefinitely.
            let output = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&step.action)
                .output()
                .await
                .map_err(|e| disyn_core::Error::Execution(e.to_string()))?;
            results.push(StepResult {
                idempotency_key: step.idempotency_key,
                step_index: i,
                success: output.status.success(),
                output: serde_json::json!({
                    "stdout": String::from_utf8_lossy(&output.stdout),
                    "stderr": String::from_utf8_lossy(&output.stderr),
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
                wall_time_ms: 0,
            },
        })
    }
}
