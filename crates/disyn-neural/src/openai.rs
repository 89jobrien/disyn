use async_trait::async_trait;
use disyn_core::ports::{FactExtractor, ProposalEngine};
use disyn_core::types::{Facts, MemoryContext, Observation, PlanDraft};
use disyn_core::{Error, Result};

use crate::shared::{extract_content, memory_section, parse_plan_draft};

// TODO: Parse confidence from the LLM response instead of using a fixed default.
const DEFAULT_CONFIDENCE: f32 = 0.5;

#[derive(Clone)]
pub struct OpenAiConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

/// Shared HTTP + auth layer for all OpenAI-compatible adapters in this crate.
pub(crate) struct OpenAiClient {
    pub(crate) config: OpenAiConfig,
    http: reqwest::Client,
}

impl OpenAiClient {
    /// Construct with auth — rejects empty api_key (use for OpenAI).
    pub(crate) fn new(config: OpenAiConfig) -> Result<Self> {
        if config.api_key.is_empty() {
            return Err(Error::Inference("OPENAI_API_KEY not set".into()));
        }
        Ok(Self {
            config,
            http: reqwest::Client::new(),
        })
    }

    /// Construct without auth — for providers like Ollama that need no key.
    pub(crate) fn new_unauthenticated(config: OpenAiConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
        }
    }

    pub(crate) async fn chat(&self, body: serde_json::Value) -> Result<serde_json::Value> {
        let resp = self
            .http
            .post(format!("{}/chat/completions", self.config.base_url))
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Inference(e.to_string()))?;
        resp.json()
            .await
            .map_err(|e| Error::Inference(e.to_string()))
    }
}

pub struct OpenAiFactExtractor {
    client: OpenAiClient,
}

impl OpenAiFactExtractor {
    #[must_use = "extractor must be used"]
    pub fn new(config: OpenAiConfig) -> Result<Self> {
        Ok(Self {
            client: OpenAiClient::new(config)?,
        })
    }
}

#[async_trait]
impl FactExtractor for OpenAiFactExtractor {
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

pub struct OpenAiProposalEngine {
    client: OpenAiClient,
}

impl OpenAiProposalEngine {
    #[must_use = "engine must be used"]
    pub fn new(config: OpenAiConfig) -> Result<Self> {
        Ok(Self {
            client: OpenAiClient::new(config)?,
        })
    }
}

#[async_trait]
impl ProposalEngine for OpenAiProposalEngine {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_extractor_requires_api_key() {
        let config = OpenAiConfig {
            api_key: String::new(),
            model: "gpt-4o".into(),
            base_url: "https://api.openai.com/v1".into(),
        };
        let result = OpenAiFactExtractor::new(config);
        assert!(result.is_err());
    }
}
