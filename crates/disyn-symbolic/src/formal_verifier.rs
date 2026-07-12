use std::collections::HashMap;

use disyn_core::ports::FormalVerifier;
use disyn_core::types::{
    ClauseCombinator, FormalSpec, GroundedExtraction, ProofVerdict, SafetyClause,
};

#[derive(Default)]
pub struct NoOpFormalVerifier;

fn evaluate_clause(clause: &SafetyClause, lookup: &HashMap<&str, bool>) -> bool {
    let direct = lookup.get(clause.fact_id.as_str()).copied().unwrap_or(true);
    match clause.combinator {
        // Leaf and And: clause passes when its fact is true (or absent — benefit of the doubt)
        ClauseCombinator::Leaf | ClauseCombinator::And => direct,
        // Not: clause passes when its fact is false (negation — "this fact must NOT hold")
        ClauseCombinator::Not => !lookup
            .get(clause.fact_id.as_str())
            .copied()
            .unwrap_or(false),
        // Or: handled at the spec level; per-clause evaluation is the same as Leaf
        ClauseCombinator::Or => direct,
    }
}

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

        let mandatory: Vec<&SafetyClause> = spec.clauses.iter().filter(|c| !c.advisory).collect();

        // Or-combinator group: the spec passes for this group if ANY clause passes.
        let or_clauses: Vec<&&SafetyClause> = mandatory
            .iter()
            .filter(|c| matches!(c.combinator, ClauseCombinator::Or))
            .collect();

        let or_violated = if or_clauses.is_empty() {
            false
        } else {
            // All Or-clauses fail → violation
            or_clauses.iter().all(|c| !evaluate_clause(c, &lookup))
        };

        let mut violated: Vec<String> = mandatory
            .iter()
            .filter(|c| !matches!(c.combinator, ClauseCombinator::Or))
            .filter(|c| !evaluate_clause(c, &lookup))
            .map(|c| c.fact_id.clone())
            .collect();

        if or_violated {
            let ids: Vec<String> = or_clauses.iter().map(|c| c.fact_id.clone()).collect();
            violated.push(format!("or-group({})", ids.join(",")));
        }

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

    fn make_extraction(facts: &[(&str, bool)]) -> GroundedExtraction {
        GroundedExtraction {
            facts: facts
                .iter()
                .map(|(id, val)| GroundedFact {
                    fact: AtomicFact {
                        id: (*id).into(),
                        query: format!("Is {id} true?"),
                        layer: 0,
                    },
                    value: *val,
                    evidence: String::new(),
                })
                .collect(),
            trajectory_span: 0..1,
        }
    }

    #[test]
    fn safe_when_all_facts_true() {
        let verifier = NoOpFormalVerifier;
        let extraction = make_extraction(&[("budget_ok", true)]);
        let spec = verifier.synthesize(&extraction);
        assert!(matches!(
            verifier.verify_spec(&spec, &extraction),
            ProofVerdict::Safe
        ));
    }

    #[test]
    fn unsafe_when_fact_false() {
        let verifier = NoOpFormalVerifier;
        let extraction = make_extraction(&[("budget_ok", false)]);
        let spec = verifier.synthesize(&extraction);
        match verifier.verify_spec(&spec, &extraction) {
            ProofVerdict::Unsafe { violated_facts } => {
                assert_eq!(violated_facts, vec!["budget_ok"]);
            }
            _ => panic!("expected Unsafe"),
        }
    }

    #[test]
    fn not_combinator_passes_when_fact_is_false() {
        let verifier = NoOpFormalVerifier;
        let extraction = make_extraction(&[("danger", false)]);
        let spec = FormalSpec {
            clauses: vec![SafetyClause {
                fact_id: "danger".into(),
                advisory: false,
                combinator: ClauseCombinator::Not,
            }],
            formal_text: None,
        };
        assert!(matches!(
            verifier.verify_spec(&spec, &extraction),
            ProofVerdict::Safe
        ));
    }

    #[test]
    fn not_combinator_violates_when_fact_is_true() {
        let verifier = NoOpFormalVerifier;
        let extraction = make_extraction(&[("danger", true)]);
        let spec = FormalSpec {
            clauses: vec![SafetyClause {
                fact_id: "danger".into(),
                advisory: false,
                combinator: ClauseCombinator::Not,
            }],
            formal_text: None,
        };
        match verifier.verify_spec(&spec, &extraction) {
            ProofVerdict::Unsafe { violated_facts } => {
                assert_eq!(violated_facts, vec!["danger"]);
            }
            _ => panic!("expected Unsafe"),
        }
    }

    #[test]
    fn or_group_passes_when_any_fact_true() {
        let verifier = NoOpFormalVerifier;
        let extraction = make_extraction(&[("a", true), ("b", false)]);
        let spec = FormalSpec {
            clauses: vec![
                SafetyClause {
                    fact_id: "a".into(),
                    advisory: false,
                    combinator: ClauseCombinator::Or,
                },
                SafetyClause {
                    fact_id: "b".into(),
                    advisory: false,
                    combinator: ClauseCombinator::Or,
                },
            ],
            formal_text: None,
        };
        assert!(matches!(
            verifier.verify_spec(&spec, &extraction),
            ProofVerdict::Safe
        ));
    }

    #[test]
    fn or_group_violates_when_all_facts_false() {
        let verifier = NoOpFormalVerifier;
        let extraction = make_extraction(&[("a", false), ("b", false)]);
        let spec = FormalSpec {
            clauses: vec![
                SafetyClause {
                    fact_id: "a".into(),
                    advisory: false,
                    combinator: ClauseCombinator::Or,
                },
                SafetyClause {
                    fact_id: "b".into(),
                    advisory: false,
                    combinator: ClauseCombinator::Or,
                },
            ],
            formal_text: None,
        };
        match verifier.verify_spec(&spec, &extraction) {
            ProofVerdict::Unsafe { violated_facts } => {
                assert!(violated_facts[0].starts_with("or-group("));
            }
            _ => panic!("expected Unsafe"),
        }
    }
}
