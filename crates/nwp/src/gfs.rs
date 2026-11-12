use anyhow::Result;
use chrono::{DateTime, Timelike, Utc};
use reqwest::Client;
use tracing::{info, warn};

use crate::grib;
use noaas_ark_core::types::Station;

/// NOAA NOMADS base URL for GFS ensemble data
const GFS_ENS_BASE: &str = "https://nomads.ncep.noaa.gov/pub/data/nccf/com/gens/prod";

/// AWS Open Data mirror (often faster, no rate limits)
pub const GFS_S3_BASE: &str = "https://noaa-gefs-pds.s3.amazonaws.com";

pub struct GfsEnsembleClient {
    http: Client,
    pub n_members: usize,
}

impl GfsEnsembleClient {
    pub fn new() -> Self {
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap(),
            n_members: 31,
        }
    }

    /// Fetch 2m temperature ensemble for a specific model run and forecast hour.
    /// Returns one temperature per ensemble member in Kelvin.
    pub async fn fetch_t2m_ensemble(
        &self,
        init_time: DateTime<Utc>,
        forecast_hour: u32,
        station: &Station,
    ) -> Result<Vec<f64>> {
        let cycle = format_cycle(init_time);
        let date = init_time.format("%Y%m%d");

        let mut temps = Vec::with_capacity(self.n_members);

        for member in 0..self.n_members {
            let member_str = if member == 0 {
                "gec00".to_string()
            } else {
                format!("gep{:02}", member)
            };

            let url = format!(
                "{}/gefs.{}/{}/atmos/pgrb2sp25/{}.t{}z.pgrb2a.0p25.f{:03}",
                GFS_ENS_BASE, date, cycle, member_str, cycle, forecast_hour
            );

            info!(member = member, url = %url, "Fetching GFS ensemble member");

            let response = self.http.get(&url).send().await?;

            if !response.status().is_success() {
                warn!(member = member, status = %response.status(), "Failed to fetch member");
                continue;
            }

            let bytes = response.bytes().await?;
            let temp_k = grib::extract_t2m_at_point(&bytes, station.lat, station.lon)?;
            temps.push(temp_k);
        }

        if temps.len() < self.n_members / 2 {
            anyhow::bail!(
                "Too few ensemble members retrieved: {}/{}",
                temps.len(),
                self.n_members
            );
        }

        Ok(temps)
    }

    /// Same as above but via the AWS S3 mirror (no rate limits).
    pub async fn fetch_t2m_ensemble_s3(
        &self,
        init_time: DateTime<Utc>,
        forecast_hour: u32,
        station: &Station,
    ) -> Result<Vec<f64>> {
        let cycle = format_cycle(init_time);
        let date = init_time.format("%Y%m%d");
        let mut temps = Vec::with_capacity(self.n_members);
        for member in 0..self.n_members {
            let member_str = if member == 0 {
                "gec00".to_string()
            } else {
                format!("gep{:02}", member)
            };
            let url = format!(
                "{}/gefs.{}/{}/{}.t{}z.pgrb2a.0p25.f{:03}",
                GFS_S3_BASE, date, cycle, member_str, cycle, forecast_hour
            );
            let response = self.http.get(&url).send().await?;
            if !response.status().is_success() {
                warn!(member = member, status = %response.status(), "S3 fetch failed");
                continue;
            }
            let bytes = response.bytes().await?;
            let temp_k = grib::extract_t2m_at_point(&bytes, station.lat, station.lon)?;
            temps.push(temp_k);
        }
        if temps.len() < self.n_members / 2 {
            anyhow::bail!(
                "Too few S3 members retrieved: {}/{}",
                temps.len(),
                self.n_members
            );
        }
        Ok(temps)
    }
}

impl Default for GfsEnsembleClient {
    fn default() -> Self {
        Self::new()
    }
}

pub fn format_cycle(dt: DateTime<Utc>) -> String {
    format!("{:02}", dt.hour())
}
