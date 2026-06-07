use disyn_core::types::{CostEstimate, ResourceUsage};

pub struct BudgetManager {
    max_tokens: u64,
    max_neural_replans: u32,
    max_repair_attempts: u32,
    used_tokens: u64,
    neural_replans: u32,
    repair_attempts: u32,
}

impl BudgetManager {
    pub fn new(max_tokens: u64, max_neural_replans: u32, max_repair_attempts: u32) -> Self {
        Self {
            max_tokens,
            max_neural_replans,
            max_repair_attempts,
            used_tokens: 0,
            neural_replans: 0,
            repair_attempts: 0,
        }
    }

    pub fn can_afford(&self, estimate: &CostEstimate) -> bool {
        let needed = u64::from(estimate.input_tokens) + u64::from(estimate.output_tokens);
        self.used_tokens + needed <= self.max_tokens
    }

    pub fn record(&mut self, usage: &ResourceUsage) {
        self.used_tokens += usage.total_tokens;
    }

    pub fn can_repair(&self) -> bool {
        self.repair_attempts < self.max_repair_attempts
    }

    pub fn record_repair(&mut self) {
        self.repair_attempts += 1;
    }

    pub fn can_replan(&self) -> bool {
        self.neural_replans < self.max_neural_replans
    }

    pub fn record_replan(&mut self) {
        self.neural_replans += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_starts_with_capacity() {
        let b = BudgetManager::new(1000, 1, 3);
        assert!(b.can_afford(&CostEstimate {
            input_tokens: 500,
            output_tokens: 200,
        }));
    }

    #[test]
    fn budget_rejects_when_exceeded() {
        let mut b = BudgetManager::new(100, 1, 3);
        b.record(&ResourceUsage {
            total_tokens: 90,
            wall_time_ms: 100,
        });
        assert!(!b.can_afford(&CostEstimate {
            input_tokens: 50,
            output_tokens: 50,
        }));
    }

    #[test]
    fn replan_attempts_decrement() {
        let mut b = BudgetManager::new(1000, 2, 3);
        assert!(b.can_replan());
        b.record_replan();
        b.record_replan();
        assert!(!b.can_replan());
    }

    #[test]
    fn repair_attempts_decrement() {
        let mut b = BudgetManager::new(1000, 1, 3);
        assert!(b.can_repair());
        b.record_repair();
        b.record_repair();
        b.record_repair();
        assert!(!b.can_repair());
    }
}
