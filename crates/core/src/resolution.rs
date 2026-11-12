// crates/core/src/resolution.rs
//
// Exact resolution criteria for a Polymarket weather market.
// Polymarket resolves on a specific station observation with specific
// rounding/time-window/source rules. Encode them to avoid silent losses.

#[derive(Debug, Clone, PartialEq)]
pub enum DataSource {
    NwsCli,
    Metar,
    Asos,
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimeWindow {
    LocalDay { timezone: String },
    UtcDay,
    Custom { start_hour: u32, end_hour: u32 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoundingRule {
    Standard,
    StrictExceed,
    Floor,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MissingDataRule {
    VoidMarket,
    UseBackupStation(String),
    Defer,
}

#[derive(Debug, Clone)]
pub struct ResolutionCriteria {
    pub station_id: String,
    pub time_window: TimeWindow,
    pub source: DataSource,
    pub rounding: RoundingRule,
    pub missing_data_rule: MissingDataRule,
}

impl ResolutionCriteria {
    /// Does an observed value (in the market's units) resolve YES for a threshold?
    pub fn resolves_yes(&self, observed: f64, threshold: f64) -> bool {
        match self.rounding {
            RoundingRule::Standard => observed.round() >= threshold,
            RoundingRule::StrictExceed => observed > threshold,
            RoundingRule::Floor => observed.floor() >= threshold,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crit(rounding: RoundingRule) -> ResolutionCriteria {
        ResolutionCriteria {
            station_id: "KNYC".into(),
            time_window: TimeWindow::LocalDay {
                timezone: "America/New_York".into(),
            },
            source: DataSource::NwsCli,
            rounding: rounding,
            missing_data_rule: MissingDataRule::VoidMarket,
        }
    }

    #[test]
    fn strict_exceed() {
        let c = crit(RoundingRule::StrictExceed);
        assert!(!c.resolves_yes(89.9, 90.0));
        assert!(!c.resolves_yes(90.0, 90.0));
        assert!(c.resolves_yes(90.0001, 90.0));
    }

    #[test]
    fn floor_rule() {
        let c = crit(RoundingRule::Floor);
        assert!(!c.resolves_yes(89.9, 90.0));
        assert!(c.resolves_yes(90.0, 90.0));
    }
}
