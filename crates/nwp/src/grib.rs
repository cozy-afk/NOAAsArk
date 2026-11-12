use anyhow::Result;
use std::io::Write;
use std::process::Command;

/// Extract 2m temperature at a lat/lon from GRIB2 bytes via wgrib2.
/// Requires `wgrib2` on PATH (v1 strategy: battle-tested external tool).
pub fn extract_t2m_at_point(grib_bytes: &[u8], lat: f64, lon: f64) -> Result<f64> {
    let mut tmp = tempfile::NamedTempFile::new()?;
    tmp.write_all(grib_bytes)?;
    let path = tmp.path().to_str().unwrap().to_string();

    let output = Command::new("wgrib2")
        .args([
            &path,
            "-match",
            ":TMP:2 m above ground:",
            "-lon",
            &format!("{:.4}", lat),
            &format!("{:.4}", lon),
            "-text",
            "/dev/stdout",
        ])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("wgrib2 failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value = stdout
        .lines()
        .find_map(|line| {
            line.split(':')
                .find(|s| s.starts_with("val="))
                .and_then(|s| s.strip_prefix("val="))
                .and_then(|v| v.parse::<f64>().ok())
        })
        .ok_or_else(|| anyhow::anyhow!("Could not parse wgrib2 output: {}", stdout))?;

    Ok(value)
}

/// Check wgrib2 availability (used at startup for a clear error).
pub fn wgrib2_available() -> bool {
    Command::new("wgrib2")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
