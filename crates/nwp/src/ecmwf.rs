//! ECMWF open data fetcher (stub for v0.1).
//! ECMWF open data: https://data.ecmwf.int/forecasts/BC/0p25/

use anyhow::Result;
use noaas_ark_core::types::Station;

pub struct EcmwfClient {
    #[allow(dead_code)]
    http: reqwest::Client,
}

impl EcmwfClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap(),
        }
    }

    pub async fn fetch_t2m_ensemble(
        &self,
        _init_time: chrono::DateTime<chrono::Utc>,
        _forecast_hour: u32,
        _station: &Station,
    ) -> Result<Vec<f64>> {
        anyhow::bail!("ECMWF fetcher not yet implemented in v0.1 - use GFS")
    }
}

impl Default for EcmwfClient {
    fn default() -> Self {
        Self::new()
    }
}
