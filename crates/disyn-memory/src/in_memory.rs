use async_trait::async_trait;
use std::sync::Mutex;

use disyn_core::Result;
use disyn_core::ports::MemoryStore;
use disyn_core::types::{ExecutionReport, Facts, MemoryContext};

// TODO: Implement a capacity limit and eviction policy to prevent unbounded memory growth.
pub struct InMemoryStore {
    episodes: Mutex<Vec<ExecutionReport>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            episodes: Mutex::new(Vec::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryStore for InMemoryStore {
    async fn retrieve(&self, _facts: &Facts) -> Result<MemoryContext> {
        // TODO: Implement semantic retrieval — score stored episodes against facts and return the
        // most relevant ones instead of always returning an empty context.
        Ok(MemoryContext {
            relevant_episodes: vec![],
            summary: None,
            weighted_passages: vec![],
        })
    }

    async fn persist(&self, report: &ExecutionReport) -> Result<()> {
        self.episodes
            .lock()
            .map_err(|e| disyn_core::Error::Memory(e.to_string()))?
            .push(report.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn retrieve_returns_empty_context_initially() {
        let store = InMemoryStore::new();
        let facts = Facts {
            entities: vec!["test".into()],
            relations: vec![],
            confidence: 1.0,
        };
        let ctx = store.retrieve(&facts).await.unwrap();
        assert!(ctx.relevant_episodes.is_empty());
        assert!(ctx.summary.is_none());
    }
}
