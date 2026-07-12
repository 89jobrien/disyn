use clap::Parser;
use disyn_core::types::Observation;
use disyn_memory::InMemoryStore;
use disyn_neural::openai::OpenAiConfig;
use disyn_neural::{
    OllamaFactExtractor, OllamaProposalEngine, OpenAiFactExtractor, OpenAiProposalEngine,
};
use disyn_runtime::{BudgetManager, ShellExecutor, TracingSink};
use disyn_symbolic::{PatternRepairEngine, RuleSetVerifier};

use disyn_app::config::Config;
use disyn_app::orchestrator::{Orchestrator, OrchestratorPorts};

const MAX_REPAIR_ATTEMPTS: u32 = 3;

#[tokio::main]
async fn main() -> disyn_core::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .json()
        .init();

    let cfg = Config::parse();
    cfg.validate()?;

    let openai_cfg = OpenAiConfig {
        api_key: cfg.api_key.clone(),
        model: cfg.model.clone(),
        base_url: if cfg.provider == "ollama" {
            "http://localhost:11434/v1".into()
        } else {
            "https://api.openai.com/v1".into()
        },
    };

    let (fact_extractor, proposal_engine): (
        Box<dyn disyn_core::ports::FactExtractor>,
        Box<dyn disyn_core::ports::ProposalEngine>,
    ) = if cfg.provider == "ollama" {
        (
            Box::new(OllamaFactExtractor::new(openai_cfg.clone())),
            Box::new(OllamaProposalEngine::new(openai_cfg)),
        )
    } else {
        (
            Box::new(OpenAiFactExtractor::new(openai_cfg.clone())?),
            Box::new(OpenAiProposalEngine::new(openai_cfg)?),
        )
    };

    let mut orchestrator = Orchestrator::new(
        OrchestratorPorts {
            fact_extractor,
            proposal_engine,
            verifier: Box::new(RuleSetVerifier::default()),
            repair_engine: Box::new(PatternRepairEngine),
            memory: Box::new(InMemoryStore::new()),
            executor: Box::new(ShellExecutor::new(cfg.step_timeout_secs)),
            telemetry: Box::new(TracingSink::init()),
        },
        BudgetManager::new(cfg.max_tokens, 1, MAX_REPAIR_ATTEMPTS, cfg.max_tokens),
    );

    let input = std::io::read_to_string(std::io::stdin())
        .map_err(|e| disyn_core::Error::Other(e.to_string()))?;
    let obs = Observation {
        source: "stdin".into(),
        payload: serde_json::from_str(&input).unwrap_or(serde_json::json!({ "raw": input })),
        timestamp: chrono::Utc::now(),
    };

    let report = orchestrator.run(obs).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|e| disyn_core::Error::Other(e.to_string()))?
    );
    Ok(())
}
