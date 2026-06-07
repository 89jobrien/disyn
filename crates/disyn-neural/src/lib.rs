pub mod ollama;
pub mod openai;

pub use ollama::{OllamaFactExtractor, OllamaProposalEngine};
pub use openai::{OpenAiConfig, OpenAiFactExtractor, OpenAiProposalEngine};
