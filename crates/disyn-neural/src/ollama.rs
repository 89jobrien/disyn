use async_trait::async_trait;
use disyn_core::Result;
use disyn_core::ports::{FactExtractor, ProposalEngine};
use disyn_core::types::{CostEstimate, Facts, MemoryContext, Observation, PlanDraft, PlannedStep};

// TODO: Parse confidence from the LLM response instead of using a fixed default.
const DEFAULT_CONFIDENCE: f32 = 0.5;

use crate::openai::{OpenAiClient, OpenAiConfig};

pub struct OllamaFactExtractor {
    client: OpenAiClient,
}

impl OllamaFactExtractor {
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
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("{}");
        // TODO: Return an error when content is missing or unparseable instead of silently
        // falling back to an empty JSON object — silent fallback produces empty Facts with no
        // indication that the model response was malformed.
        let parsed: serde_json::Value =
            serde_json::from_str(content).unwrap_or(serde_json::json!({}));
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
    pub fn new(config: OpenAiConfig) -> Self {
        Self {
            client: OpenAiClient::new_unauthenticated(config),
        }
    }
}

#[async_trait]
impl ProposalEngine for OllamaProposalEngine {
    async fn propose(&self, facts: &Facts, memory: &MemoryContext) -> Result<PlanDraft> {
        let memory_section = match &memory.summary {
            Some(s) if !s.is_empty() => format!("\n\nPrior context:\n{s}"),
            _ => String::new(),
        };
        let body = serde_json::json!({
            "model": self.client.config.model,
            "messages": [{
                "role": "user",
                "content": format!(
                    "Given these facts, propose a plan: {:?}{}",
                    facts.entities,
                    memory_section,
                ),
            }],
            "response_format": { "type": "json_object" },
        });
        let data = self.client.chat(body).await?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("{}");
        let parsed: serde_json::Value =
            serde_json::from_str(content).unwrap_or(serde_json::json!({}));
        let steps = parsed["steps"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|s| PlannedStep {
                        idempotency_key: uuid::Uuid::new_v4(),
                        action: s["action"].as_str().unwrap_or("unknown").to_string(),
                        parameters: s["parameters"].clone(),
                        estimated_cost: CostEstimate {
                            class: Some(disyn_core::types::CostClass::Neural),
                            input_tokens: 0,
                            output_tokens: 0,
                        },
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(PlanDraft {
            steps,
            rationale: parsed["rationale"]
                .as_str()
                .unwrap_or("LLM-generated plan")
                .to_string(),
        })
    }
}
