pub mod ollama;
pub mod openai;
mod shared;

pub use ollama::{OllamaFactExtractor, OllamaProposalEngine};
pub use openai::{OpenAiConfig, OpenAiFactExtractor, OpenAiProposalEngine};
