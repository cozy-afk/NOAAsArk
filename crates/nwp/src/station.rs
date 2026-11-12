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
                id: "KNYC".to_string(),
                name: "Central Park, NYC".to_string(),
                lat: 40.7831,
                lon: -73.9662,
                elevation_m: 42.0,
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
        ];
        for s in defaults {
            stations.insert(s.id.clone(), s);
        }
        Self { stations }
    }

    pub fn get(&self, id: &str) -> Option<&Station> {
        self.stations.get(id)
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
