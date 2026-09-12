//! LSR (Landskrona-Svalövs Renhållnings AB) — publik REST-fasad på
//! `minasidor.lsr.nu/api/api/external/`. Widgeten på lsr.nu/hamtningsschema/
//! anropar två endpoints anonymt:
//!
//!   POST /autocompleteAllPost/  body {"Address": "<query>"}
//!     → ["STREET N, CITY", ...]
//!   POST /schedulePost/         body {"Address": "<selection>"}
//!     → [{ date: "YYYY-MM-DDT00:00:00", typeOfWasteDescription, ... }]
//!
//! schedulePost returnerar hela årsplanen som explicit datum-lista per
//! fraktion — vi grupperar efter waste_type och emitterar en
//! PickupSeries per grupp med explicit anchor-datum (ingen RRULE
//! trots regelbunden `collectionFrequency`).

use async_trait::async_trait;
use chrono::NaiveDate;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const BASE: &str = "https://minasidor.lsr.nu/api/api/external";

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    pub cities: &'static [&'static str],
}

pub struct Lsr {
    http: Client,
    cfg: Config,
}

impl Lsr {
    pub fn new(http: Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    fn matches_city(&self, city: &str) -> bool {
        let needle = city.trim().to_lowercase();
        self.cfg.cities.iter().any(|c| c.to_lowercase() == needle)
    }
}

#[derive(Debug, Serialize)]
struct AddressBody<'a> {
    #[serde(rename = "Address")]
    address: &'a str,
}

#[derive(Debug, Deserialize)]
struct ScheduleEntry {
    date: String,
    #[serde(rename = "typeOfWasteDescription", default)]
    type_of_waste_description: Option<String>,
    #[serde(rename = "collectionFrequency", default)]
    collection_frequency: Option<String>,
}

#[async_trait]
impl Provider for Lsr {
    fn id(&self) -> &'static str {
        self.cfg.id
    }
    fn name(&self) -> &'static str {
        self.cfg.name
    }
    fn placeholder(&self) -> &'static str {
        self.cfg.placeholder
    }
    fn note(&self) -> &'static str {
        self.cfg.note
    }

    async fn autocomplete(&self, query: &str) -> Result<Vec<Suggestion>, ProviderError> {
        let q = query.trim();
        if q.len() < 2 {
            return Ok(vec![]);
        }
        let raw: Vec<String> = self
            .http
            .post(format!("{BASE}/autocompleteAllPost/"))
            .header("Referer", "https://www.lsr.nu/hamtningsschema/")
            .json(&AddressBody { address: q })
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;

        let mut seen = std::collections::HashSet::new();
        Ok(raw
            .into_iter()
            .filter(|s| {
                let city = city_of(s);
                self.matches_city(city) && seen.insert(s.clone())
            })
            .map(|value| Suggestion { value })
            .collect())
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let entries: Vec<ScheduleEntry> = self
            .http
            .post(format!("{BASE}/schedulePost/"))
            .header("Referer", "https://www.lsr.nu/hamtningsschema/")
            .json(&AddressBody { address })
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;

        Ok(build_schedule(address.to_string(), entries))
    }
}

fn city_of(building: &str) -> &str {
    building
        .rsplit_once(", ")
        .map(|(_, c)| c.trim())
        .unwrap_or(building)
}

fn build_schedule(address: String, entries: Vec<ScheduleEntry>) -> PickupSchedule {
    let mut grouped: BTreeMap<String, (Option<String>, Vec<NaiveDate>)> = BTreeMap::new();
    for e in entries {
        let Some(waste) = e.type_of_waste_description else {
            continue;
        };
        let Some(date) = parse_date(&e.date) else {
            continue;
        };
        let slot = grouped.entry(waste).or_insert_with(|| (e.collection_frequency.clone(), Vec::new()));
        if !slot.1.contains(&date) {
            slot.1.push(date);
        }
        // Keep first non-empty frequency code.
        if slot.0.is_none() {
            slot.0 = e.collection_frequency;
        }
    }

    let series = grouped
        .into_iter()
        .map(|(waste_type, (freq_code, mut dates))| {
            dates.sort();
            PickupSeries {
                waste_type,
                frequency_text: describe_frequency(freq_code.as_deref()),
                interval_weeks: None,
                anchor: dates,
            }
        })
        .collect();

    PickupSchedule { address, series }
}

fn parse_date(iso: &str) -> Option<NaiveDate> {
    // Server returns "YYYY-MM-DDT00:00:00" (no timezone). Truncate to date.
    let date_part = iso.get(0..10)?;
    NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()
}

fn describe_frequency(code: Option<&str>) -> String {
    // Format: `I<per-year>`. I026 = 26/year (varannan vecka), I013 = 13/year
    // (var fjärde vecka), I052 = varje vecka, etc.
    let Some(c) = code else {
        return "LSR-schema".into();
    };
    let stripped = c.trim().trim_start_matches(|ch: char| !ch.is_ascii_digit());
    match stripped.parse::<u32>().ok() {
        Some(52) => "Varje vecka".into(),
        Some(26) => "Varannan vecka".into(),
        Some(13) => "Var fjärde vecka".into(),
        Some(6) => "Var 8:e vecka".into(),
        Some(4) => "Var 13:e vecka".into(),
        Some(n) => format!("{n} gånger per år"),
        None => c.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn city_of_extracts_last_segment() {
        assert_eq!(city_of("STORGATAN 12, LANDSKRONA"), "LANDSKRONA");
        assert_eq!(city_of("STORGATAN 30-32,    5032, LANDSKRONA"), "LANDSKRONA");
    }

    #[test]
    fn parse_date_handles_iso_timestamp() {
        assert_eq!(
            parse_date("2026-09-21T00:00:00"),
            Some(NaiveDate::from_ymd_opt(2026, 9, 21).unwrap())
        );
        assert!(parse_date("invalid").is_none());
    }

    #[test]
    fn describe_frequency_maps_common_codes() {
        assert_eq!(describe_frequency(Some("I026")), "Varannan vecka");
        assert_eq!(describe_frequency(Some("I013")), "Var fjärde vecka");
        assert_eq!(describe_frequency(Some("I052")), "Varje vecka");
        assert_eq!(describe_frequency(Some("I100")), "100 gånger per år");
        assert_eq!(describe_frequency(None), "LSR-schema");
    }

    #[test]
    fn build_schedule_groups_by_waste_type() {
        let entries = vec![
            ScheduleEntry {
                date: "2026-09-21T00:00:00".into(),
                type_of_waste_description: Some("Restavfall".into()),
                collection_frequency: Some("I026".into()),
            },
            ScheduleEntry {
                date: "2026-10-05T00:00:00".into(),
                type_of_waste_description: Some("Restavfall".into()),
                collection_frequency: Some("I026".into()),
            },
            ScheduleEntry {
                date: "2026-10-05T00:00:00".into(),
                type_of_waste_description: Some("Förpackningar".into()),
                collection_frequency: Some("I013".into()),
            },
        ];
        let s = build_schedule("Storgatan 12, Landskrona".into(), entries);
        assert_eq!(s.series.len(), 2);
        let rest = s.series.iter().find(|p| p.waste_type == "Restavfall").unwrap();
        assert_eq!(rest.anchor.len(), 2);
        assert_eq!(rest.frequency_text, "Varannan vecka");
        let fp = s.series.iter().find(|p| p.waste_type == "Förpackningar").unwrap();
        assert_eq!(fp.anchor.len(), 1);
        assert_eq!(fp.frequency_text, "Var fjärde vecka");
    }

    #[test]
    fn build_schedule_dedups_dates_within_same_type() {
        let entries = vec![
            ScheduleEntry {
                date: "2026-09-21T00:00:00".into(),
                type_of_waste_description: Some("Restavfall".into()),
                collection_frequency: Some("I026".into()),
            },
            ScheduleEntry {
                date: "2026-09-21T00:00:00".into(),
                type_of_waste_description: Some("Restavfall".into()),
                collection_frequency: Some("I026".into()),
            },
        ];
        let s = build_schedule("x".into(), entries);
        assert_eq!(s.series.len(), 1);
        assert_eq!(s.series[0].anchor.len(), 1);
    }
}
