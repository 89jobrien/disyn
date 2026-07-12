use disyn_core::types::{CostClass, CostEstimate, MemoryContext, PlanDraft, PlannedStep};
use disyn_core::{Error, Result};

pub(crate) fn memory_section(memory: &MemoryContext) -> String {
    match &memory.summary {
        Some(s) if !s.is_empty() => format!("\n\nPrior context:\n{s}"),
        _ => String::new(),
    }
}

pub(crate) fn extract_content(data: &serde_json::Value) -> Result<serde_json::Value> {
    let raw = data["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| Error::Inference("LLM response missing content field".into()))?;
    serde_json::from_str(raw)
        .map_err(|e| Error::Inference(format!("LLM response is not valid JSON: {e}")))
}

pub(crate) fn parse_plan_draft(data: &serde_json::Value) -> Result<PlanDraft> {
    let parsed = extract_content(data)?;
    let steps = parsed["steps"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|s| PlannedStep {
                    idempotency_key: uuid::Uuid::new_v4(),
                    action: s["action"].as_str().unwrap_or("unknown").to_string(),
                    parameters: s["parameters"].clone(),
                    estimated_cost: CostEstimate {
                        class: Some(CostClass::Neural),
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
