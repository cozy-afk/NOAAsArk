//! HRRR short-range fetcher (stub for v0.1).
//! HRRR: https://nomads.ncep.noaa.gov/pub/data/nccf/com/hrrr/prod/

use anyhow::Result;
use noaas_ark_core::types::Station;

pub struct HrrrClient {
    #[allow(dead_code)]
    http: reqwest::Client,
}

impl HrrrClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap(),
        }
    }

    pub async fn fetch_t2m(
        &self,
        _init_time: chrono::DateTime<chrono::Utc>,
        _forecast_hour: u32,
        _station: &Station,
    ) -> Result<f64> {
        anyhow::bail!("HRRR fetcher not yet implemented in v0.1 - use GFS")
    }
}

impl Default for HrrrClient {
    fn default() -> Self {
        Self::new()
    }
}
