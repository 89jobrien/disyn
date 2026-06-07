use async_trait::async_trait;
use std::sync::Mutex;

use disyn_core::Result;
use disyn_core::ports::MemoryStore;
use disyn_core::types::{ExecutionReport, Facts, MemoryContext};

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
        Ok(MemoryContext {
            relevant_episodes: vec![],
            summary: None,
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
