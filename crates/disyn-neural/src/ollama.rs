use async_trait::async_trait;
use disyn_core::ports::{FactExtractor, ProposalEngine};
use disyn_core::types::{CostEstimate, Facts, MemoryContext, Observation, PlanDraft, PlannedStep};
use disyn_core::{Error, Result};

const DEFAULT_CONFIDENCE: f32 = 0.5;

use crate::openai::OpenAiConfig;

pub struct OllamaFactExtractor {
    config: OpenAiConfig,
    client: reqwest::Client,
}

impl OllamaFactExtractor {
    pub fn new(config: OpenAiConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl FactExtractor for OllamaFactExtractor {
    async fn extract(&self, observation: &Observation) -> Result<Facts> {
        let body = serde_json::json!({
            "model": self.config.model,
            "messages": [{
                "role": "user",
                "content": format!(
                    "Extract entities and relations from: {}",
                    observation.payload
                ),
            }],
            "response_format": { "type": "json_object" },
        });
        let resp = self
            .client
            .post(format!("{}/chat/completions", self.config.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("{}");
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
    config: OpenAiConfig,
    client: reqwest::Client,
}

impl OllamaProposalEngine {
    pub fn new(config: OpenAiConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ProposalEngine for OllamaProposalEngine {
    async fn propose(&self, facts: &Facts, _memory: &MemoryContext) -> Result<PlanDraft> {
        let body = serde_json::json!({
            "model": self.config.model,
            "messages": [{
                "role": "user",
                "content": format!(
                    "Given these facts, propose a plan: {:?}",
                    facts.entities
                ),
            }],
            "response_format": { "type": "json_object" },
        });
        let resp = self
            .client
            .post(format!("{}/chat/completions", self.config.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;
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
