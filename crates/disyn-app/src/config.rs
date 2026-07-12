use clap::Parser;
use disyn_core::{Error, Result};

#[derive(Parser)]
#[command(name = "disyn", version, about = "Hybrid agent pipeline")]
pub struct Config {
    /// LLM provider: openai or ollama
    #[arg(long, env = "DISYN_PROVIDER", default_value = "openai")]
    pub provider: String,

    /// OpenAI API key
    #[arg(long, env = "OPENAI_API_KEY", default_value = "")]
    pub api_key: String,

    /// Model name
    #[arg(long, env = "DISYN_MODEL", default_value = "gpt-4o")]
    pub model: String,

    /// Max token budget
    #[arg(long, env = "DISYN_MAX_TOKENS", default_value = "10000")]
    pub max_tokens: u64,

    /// Per-step shell timeout in seconds
    #[arg(long, env = "DISYN_STEP_TIMEOUT_SECS", default_value = "30")]
    pub step_timeout_secs: u64,
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.provider == "openai" && self.api_key.is_empty() {
            return Err(Error::Other(
                "OPENAI_API_KEY must be set when provider is 'openai'".into(),
            ));
        }
        Ok(())
    }
}
