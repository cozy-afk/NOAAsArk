// crates/engine/src/timing.rs

pub struct MarketReactionModel {
    pub half_life_minutes: f64,
    pub initial_jump_fraction: f64,
}

impl MarketReactionModel {
    pub fn expected_remaining_edge(&self, minutes_since_availability: f64) -> f64 {
        if self.half_life_minutes <= 0.0 {
            return 0.0;
        }
        (-0.693 * minutes_since_availability / self.half_life_minutes).exp()
    }

    pub fn should_trade(
        &self,
        minutes_since_availability: f64,
        raw_edge: f64,
        min_profitable_edge: f64,
    ) -> bool {
        let remaining = raw_edge * self.expected_remaining_edge(minutes_since_availability);
        remaining > min_profitable_edge
    }
}

impl Default for MarketReactionModel {
    fn default() -> Self {
        Self {
            half_life_minutes: 30.0,
            initial_jump_fraction: 0.4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay() {
        let m = MarketReactionModel::default();
        assert!(m.expected_remaining_edge(0.0) > m.expected_remaining_edge(30.0));
        assert!(m.expected_remaining_edge(0.0) <= 1.0 + 1e-9);
    }
}
