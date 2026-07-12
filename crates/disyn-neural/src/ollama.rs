use async_trait::async_trait;
use disyn_core::Result;
use disyn_core::ports::{FactExtractor, ProposalEngine};
use disyn_core::types::{Facts, MemoryContext, Observation, PlanDraft};

use crate::openai::{OpenAiClient, OpenAiConfig};
use crate::shared::{extract_content, memory_section, parse_plan_draft};

// TODO: Parse confidence from the LLM response instead of using a fixed default.
const DEFAULT_CONFIDENCE: f32 = 0.5;

pub struct OllamaFactExtractor {
    client: OpenAiClient,
}

impl OllamaFactExtractor {
    #[must_use = "extractor must be used"]
    pub fn new(config: OpenAiConfig) -> Self {
        Self {
            client: OpenAiClient::new_unauthenticated(config),
        }
    }
}

#[async_trait]
impl FactExtractor for OllamaFactExtractor {
    async fn extract(&self, observation: &Observation) -> Result<Facts> {
        let body = serde_json::json!({
            "model": self.client.config.model,
            "messages": [{
                "role": "user",
                "content": format!(
                    "Extract entities and relations from: {}",
                    observation.payload
                ),
            }],
            "response_format": { "type": "json_object" },
        });
        let data = self.client.chat(body).await?;
        let parsed = extract_content(&data)?;
        Ok(Facts {
            entities: parsed["entities"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            relations: vec![],
            confidence: DEFAULT_CONFIDENCE,
        })
    }
}

pub struct OllamaProposalEngine {
    client: OpenAiClient,
}

impl OllamaProposalEngine {
    #[must_use = "engine must be used"]
    pub fn new(config: OpenAiConfig) -> Self {
        Self {
            client: OpenAiClient::new_unauthenticated(config),
        }
    }
}

#[async_trait]
impl ProposalEngine for OllamaProposalEngine {
    async fn propose(&self, facts: &Facts, memory: &MemoryContext) -> Result<PlanDraft> {
        let body = serde_json::json!({
            "model": self.client.config.model,
            "messages": [{
                "role": "user",
                "content": format!(
                    "Given these facts, propose a plan: {:?}{}",
                    facts.entities,
                    memory_section(memory),
                ),
            }],
            "response_format": { "type": "json_object" },
        });
        let data = self.client.chat(body).await?;
        parse_plan_draft(&data)
    }
}
