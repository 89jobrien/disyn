use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("inference failed: {0}")]
    Inference(String),

    #[error("verification failed: {violations} violations")]
    Verification { violations: usize },

    #[error("repair exhausted after {attempts} attempts")]
    RepairExhausted { attempts: u32 },

    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),

    #[error("memory store: {0}")]
    Memory(String),

    #[error("execution failed: {0}")]
    Execution(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
