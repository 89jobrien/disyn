use clap::Parser;

#[derive(Parser)]
#[command(name = "disyn", version, about = "Hybrid agent pipeline")]
pub struct Config {
    /// LLM provider: openai or ollama
    #[arg(long, env = "DISYN_PROVIDER", default_value = "openai")]
    pub provider: String,

    /// OpenAI API key
    // TODO: Validate that api_key is non-empty at startup when provider == "openai" and emit a
    // clear error rather than propagating a mid-run Inference error.
    #[arg(long, env = "OPENAI_API_KEY", default_value = "")]
    pub api_key: String,

    /// Model name
    #[arg(long, env = "DISYN_MODEL", default_value = "gpt-4o")]
    pub model: String,

    /// Max token budget
    #[arg(long, env = "DISYN_MAX_TOKENS", default_value = "10000")]
    pub max_tokens: u64,
}
