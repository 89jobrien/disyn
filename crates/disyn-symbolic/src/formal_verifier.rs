use std::collections::HashMap;

use disyn_core::ports::FormalVerifier;
use disyn_core::types::{
    ClauseCombinator, FormalSpec, GroundedExtraction, ProofVerdict, SafetyClause,
};

#[derive(Default)]
pub struct NoOpFormalVerifier;

impl FormalVerifier for NoOpFormalVerifier {
    fn synthesize(&self, extraction: &GroundedExtraction) -> FormalSpec {
        let clauses = extraction
            .facts
            .iter()
            .map(|gf| SafetyClause {
                fact_id: gf.fact.id.clone(),
                advisory: false,
                combinator: ClauseCombinator::Leaf,
            })
            .collect();
        FormalSpec {
            clauses,
            formal_text: None,
        }
    }

    fn verify_spec(&self, spec: &FormalSpec, extraction: &GroundedExtraction) -> ProofVerdict {
        let lookup: HashMap<&str, bool> = extraction
            .facts
            .iter()
            .map(|gf| (gf.fact.id.as_str(), gf.value))
            .collect();

        // TODO: Handle compound ClauseCombinator variants (And, Or, Not) — currently only Leaf
        // clauses are evaluated; tree combinators are silently treated as always-satisfied.
        let violated: Vec<String> = spec
            .clauses
            .iter()
            .filter(|c| !c.advisory)
            .filter(|c| !lookup.get(c.fact_id.as_str()).copied().unwrap_or(true))
            .map(|c| c.fact_id.clone())
            .collect();

        if violated.is_empty() {
            ProofVerdict::Safe
        } else {
            ProofVerdict::Unsafe {
                violated_facts: violated,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use disyn_core::types::{AtomicFact, GroundedFact};

    #[test]
    fn safe_when_all_facts_true() {
        let verifier = NoOpFormalVerifier;
        let extraction = GroundedExtraction {
            facts: vec![GroundedFact {
                fact: AtomicFact {
                    id: "budget_ok".into(),
                    query: "Was budget respected?".into(),
                    layer: 6,
                },
                value: true,
                evidence: "spent 50 of 100 tokens".into(),
            }],
            trajectory_span: 0..1,
        };
        let spec = verifier.synthesize(&extraction);
        let verdict = verifier.verify_spec(&spec, &extraction);
        assert!(matches!(verdict, ProofVerdict::Safe));
    }

    #[test]
    fn unsafe_when_fact_false() {
        let verifier = NoOpFormalVerifier;
        let extraction = GroundedExtraction {
            facts: vec![GroundedFact {
                fact: AtomicFact {
                    id: "budget_ok".into(),
                    query: "Was budget respected?".into(),
                    layer: 6,
                },
                value: false,
                evidence: "spent 150 of 100 tokens".into(),
            }],
            trajectory_span: 0..1,
        };
        let spec = verifier.synthesize(&extraction);
        let verdict = verifier.verify_spec(&spec, &extraction);
        match verdict {
            ProofVerdict::Unsafe { violated_facts } => {
                assert_eq!(violated_facts, vec!["budget_ok"]);
            }
            _ => panic!("expected Unsafe"),
        }
    }
}
