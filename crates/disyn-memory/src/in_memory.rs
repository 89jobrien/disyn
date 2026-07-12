use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Mutex;

use disyn_core::Result;
use disyn_core::ports::MemoryStore;
use disyn_core::types::{ExecutionReport, Facts, MemoryContext, WeightedPassage};

const MAX_EPISODES: usize = 256;
const RETRIEVE_TOP_K: usize = 5;
const MIN_SCORE: f32 = 0.0;

pub struct InMemoryStore {
    episodes: Mutex<VecDeque<ExecutionReport>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            episodes: Mutex::new(VecDeque::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

fn score_episode(report: &ExecutionReport, facts: &Facts) -> f32 {
    if facts.entities.is_empty() {
        return MIN_SCORE;
    }
    let haystack: String = report
        .results
        .iter()
        .map(|r| r.output.to_string())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();

    let hits = facts
        .entities
        .iter()
        .filter(|e| haystack.contains(e.to_lowercase().as_str()))
        .count();

    hits as f32 / facts.entities.len() as f32
}

#[async_trait]
impl MemoryStore for InMemoryStore {
    async fn retrieve(&self, facts: &Facts) -> Result<MemoryContext> {
        let episodes = self
            .episodes
            .lock()
            .map_err(|e| disyn_core::Error::Memory(e.to_string()))?;

        let mut scored: Vec<(f32, usize)> = episodes
            .iter()
            .enumerate()
            .map(|(i, ep)| (score_episode(ep, facts), i))
            .filter(|(score, _)| *score > MIN_SCORE)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(RETRIEVE_TOP_K);

        let passages: Vec<WeightedPassage> = scored
            .iter()
            .map(|(score, idx)| WeightedPassage {
                content: episodes[*idx]
                    .results
                    .iter()
                    .map(|r| r.output.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                source_episode_index: *idx,
                triple_match_score: *score,
                final_weight: *score * facts.confidence,
            })
            .collect();

        let summary = if passages.is_empty() {
            None
        } else {
            Some(format!(
                "Retrieved {} relevant episode(s) matching entities: {}",
                passages.len(),
                facts.entities.join(", ")
            ))
        };

        Ok(MemoryContext {
            relevant_episodes: vec![],
            summary,
            weighted_passages: passages,
        })
    }

    async fn persist(&self, report: &ExecutionReport) -> Result<()> {
        let mut episodes = self
            .episodes
            .lock()
            .map_err(|e| disyn_core::Error::Memory(e.to_string()))?;
        if episodes.len() >= MAX_EPISODES {
            episodes.pop_front();
        }
        episodes.push_back(report.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use disyn_core::types::{ResourceUsage, StepResult};
    use uuid::Uuid;

    fn make_report(outputs: &[&str]) -> ExecutionReport {
        ExecutionReport {
            results: outputs
                .iter()
                .enumerate()
                .map(|(i, o)| StepResult {
                    idempotency_key: Uuid::new_v4(),
                    step_index: i,
                    success: true,
                    output: serde_json::json!({ "stdout": o }),
                    error: None,
                })
                .collect(),
            total_cost: ResourceUsage {
                total_tokens: 0,
                symbolic_tokens: 0,
                neural_tokens: 0,
                wall_time_ms: 0,
            },
        }
    }

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

    #[tokio::test]
    async fn retrieve_scores_matching_episodes() {
        let store = InMemoryStore::new();
        store
            .persist(&make_report(&["processed alice checkout"]))
            .await
            .unwrap();
        store
            .persist(&make_report(&["unrelated output"]))
            .await
            .unwrap();

        let facts = Facts {
            entities: vec!["alice".into(), "checkout".into()],
            relations: vec![],
            confidence: 1.0,
        };
        let ctx = store.retrieve(&facts).await.unwrap();
        assert!(ctx.summary.is_some());
        assert_eq!(ctx.weighted_passages.len(), 1);
        assert!((ctx.weighted_passages[0].triple_match_score - 1.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn evicts_oldest_when_capacity_exceeded() {
        let store = InMemoryStore::new();
        for i in 0..=MAX_EPISODES {
            store
                .persist(&make_report(&[&format!("step {i}")]))
                .await
                .unwrap();
        }
        let episodes = store.episodes.lock().unwrap();
        assert_eq!(episodes.len(), MAX_EPISODES);
    }
}
