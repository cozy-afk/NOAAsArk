use noaas_ark_core::types::Station;
use std::collections::HashMap;

/// Known verification stations (major Polymarket weather cities).
pub struct StationRegistry {
    stations: HashMap<String, Station>,
}

impl StationRegistry {
    pub fn new() -> Self {
        let mut stations = HashMap::new();
        let defaults = vec![
            Station {
                // Verified: NYC daily markets resolve on LaGuardia (KLGA) METAR.
                id: "KLGA".to_string(),
                name: "New York LaGuardia".to_string(),
                lat: 40.7769,
                lon: -73.874,
                elevation_m: 7.0,
            },
            Station {
                id: "KORD".to_string(),
                name: "Chicago O'Hare".to_string(),
                lat: 41.9742,
                lon: -87.9073,
                elevation_m: 205.0,
            },
            Station {
                id: "KLAX".to_string(),
                name: "Los Angeles Intl".to_string(),
                lat: 33.9425,
                lon: -118.4081,
                elevation_m: 38.0,
            },
            Station {
                id: "KIAH".to_string(),
                name: "Houston Intercontinental".to_string(),
                lat: 29.9844,
                lon: -95.3414,
                elevation_m: 30.0,
            },
            Station {
                id: "KPHX".to_string(),
                name: "Phoenix Sky Harbor".to_string(),
                lat: 33.4343,
                lon: -112.0116,
                elevation_m: 346.0,
            },
            Station {
                id: "KMIA".to_string(),
                name: "Miami Intl".to_string(),
                lat: 25.7959,
                lon: -80.287,
                elevation_m: 2.0,
            },
            Station {
                id: "KDFW".to_string(),
                name: "Dallas-Fort Worth Intl".to_string(),
                lat: 32.8998,
                lon: -97.0403,
                elevation_m: 185.0,
            },
            Station {
                id: "KSEA".to_string(),
                name: "Seattle-Tacoma Intl".to_string(),
                lat: 47.4502,
                lon: -122.3088,
                elevation_m: 132.0,
            },
            Station {
                id: "KDEN".to_string(),
                name: "Denver Intl".to_string(),
                lat: 39.8561,
                lon: -104.6737,
                elevation_m: 1655.0,
            },
            Station {
                id: "KATL".to_string(),
                name: "Atlanta Hartsfield".to_string(),
                lat: 33.6407,
                lon: -84.4277,
                elevation_m: 313.0,
            },
            Station {
                // Verified: London daily markets resolve on London City (EGLC) METAR.
                id: "EGLC".to_string(),
                name: "London City Airport".to_string(),
                lat: 51.5053,
                lon: 0.0553,
                elevation_m: 6.0,
            },
            Station {
                // Verified: Paris daily markets resolve on Le Bourget (LFPB) METAR.
                id: "LFPB".to_string(),
                name: "Paris Le Bourget".to_string(),
                lat: 48.9694,
                lon: 2.4414,
                elevation_m: 66.0,
            },
        ];
        for s in defaults {
            stations.insert(s.id.clone(), s);
        }
        Self { stations }
    }

    pub fn get(&self, id: &str) -> Option<&Station> {
        self.stations.get(id)
    }

    /// Match a Polymarket question city name ("New York City", "Chicago", ...)
    /// to a verification station. Case-insensitive; returns None for unknown.
    pub fn lookup_by_city_name(&self, city: &str) -> Option<&Station> {
        let normalized = city.trim().to_lowercase();
        let id = match normalized.as_str() {
            "new york city" | "nyc" | "new york" => "KLGA",
            "chicago" => "KORD",
            "los angeles" | "la" => "KLAX",
            "houston" => "KIAH",
            "phoenix" => "KPHX",
            "miami" => "KMIA",
            "dallas" | "dallas-fort worth" | "fort worth" => "KDFW",
            "seattle" => "KSEA",
            "denver" => "KDEN",
            "atlanta" => "KATL",
            "london" => "EGLC",
            "paris" => "LFPB",
            _ => return None,
        };
        self.get(id)
    }

    pub fn register(&mut self, station: Station) {
        self.stations.insert(station.id.clone(), station);
    }

    pub fn all(&self) -> Vec<&Station> {
        self.stations.values().collect()
    }
}

impl Default for StationRegistry {
    fn default() -> Self {
        Self::new()
    }
}
