//! Provider for kommuner using EXDE Systems' Mina sidor portal. Two
//! deployment topologies seen in the wild, both with the same JSON
//! contract:
//!
//!   Pattern A — Azure-hosted, per-kommun subdomain:
//!     POST https://minasidor-<slug>-az.exdesystems.se/api/api/external/autocompleteAllPost
//!     POST https://minasidor-<slug>-az.exdesystems.se/api/api/external/schedulePost
//!
//!   Pattern B — own domain, MinaSidor_API prefix (used by Ökrab):
//!     POST https://minasidor.<host>.se/MinaSidor_API/api/external/autocompleteAllPost
//!     POST https://minasidor.<host>.se/MinaSidor_API/api/external/schedulePost
//!
//! Both endpoints accept `{"Address": "..."}` JSON body. The autocomplete
//! returns a flat array of uppercase address strings ("STORGATAN 1, ORT");
//! the schedule returns an array of `{date, title, typeOfWaste,
//! collectionFrequency, ...}` per pickup occurrence.
//!
//! Confirmed deployments: Danderyd, Täby (Azure pattern); Simrishamn,
//! Tomelilla (both via Ökrab — same backend filtered by locality).

use std::collections::{BTreeMap, HashSet};

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    /// Trailing slash stripped before use. The provider appends
    /// `/autocompleteAllPost` and `/schedulePost`.
    pub api_url: &'static str,
    /// `None` for single-kommun deployments. For multi-kommun bolag,
    /// list the localities that belong to this kommun (case-insensitive).
    pub cities: Option<&'static [&'static str]>,
}

pub struct Exde {
    http: reqwest::Client,
    cfg: Config,
}

impl Exde {
    pub fn new(http: reqwest::Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    fn api(&self) -> &str {
        self.cfg.api_url.trim_end_matches('/')
    }

    async fn autocomplete_raw(&self, query: &str) -> Result<Vec<String>, ProviderError> {
        let url = format!("{}/autocompleteAllPost", self.api());
        let resp: Vec<String> = self
            .http
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&json!({"Address": query}))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp)
    }

    async fn fetch_schedule(&self, address: &str) -> Result<Vec<ScheduleEntry>, ProviderError> {
        let url = format!("{}/schedulePost", self.api());
        let resp: Vec<ScheduleEntry> = self
            .http
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&json!({"Address": address}))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp)
    }

    fn matches_city(&self, city: &str) -> bool {
        match self.cfg.cities {
            None => true,
            Some(list) => {
                let needle = city.to_lowercase();
                list.iter().any(|c| c.to_lowercase() == needle)
            }
        }
    }
}

#[derive(Deserialize, Debug)]
struct ScheduleEntry {
    #[serde(default)]
    date: String,
    #[serde(default, rename = "typeOfWasteDescription")]
    type_of_waste_description: String,
    #[serde(default, rename = "wasteType")]
    waste_type: String,
    #[serde(default, rename = "typeOfWaste")]
    type_of_waste: String,
    #[serde(default)]
    title: String,
    #[serde(default, rename = "collectionFrequency")]
    collection_frequency: String,
}

#[async_trait]
impl Provider for Exde {
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
        let raw = self.autocomplete_raw(q).await?;
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for value in raw {
            let city = value.rsplit(',').next().unwrap_or("").trim();
            if !self.matches_city(city) {
                continue;
            }
            if seen.insert(value.clone()) {
                out.push(Suggestion { value });
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let entries = self.fetch_schedule(address).await?;
        let series = build_series(&entries);
        Ok(PickupSchedule {
            address: address.to_string(),
            series,
        })
    }
}

fn build_series(entries: &[ScheduleEntry]) -> Vec<PickupSeries> {
    // (waste_type_label) -> (frequency_text, frequency_code, dates)
    let mut by_type: BTreeMap<String, (String, String, Vec<NaiveDate>)> = BTreeMap::new();
    for e in entries {
        let Some(date) = parse_date(&e.date) else {
            continue;
        };
        let label = waste_type_label(e);
        if label.is_empty() {
            continue;
        }
        let entry = by_type.entry(label).or_insert_with(|| {
            (e.title.clone(), e.collection_frequency.clone(), Vec::new())
        });
        if entry.0.is_empty() {
            entry.0 = e.title.clone();
        }
        if entry.1.is_empty() {
            entry.1 = e.collection_frequency.clone();
        }
        entry.2.push(date);
    }

    let mut series = Vec::new();
    for (waste_type, (frequency_text, freq_code, mut dates)) in by_type {
        dates.sort();
        dates.dedup();
        let interval_weeks = interval_from_frequency(&freq_code);
        let anchor = if interval_weeks.is_some() {
            dates.into_iter().take(1).collect()
        } else {
            dates
        };
        series.push(PickupSeries {
            waste_type,
            frequency_text,
            interval_weeks,
            anchor,
        });
    }
    series
}

fn waste_type_label(e: &ScheduleEntry) -> String {
    if !e.type_of_waste_description.is_empty() {
        e.type_of_waste_description.clone()
    } else if !e.waste_type.is_empty() {
        e.waste_type.clone()
    } else {
        e.type_of_waste.clone()
    }
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // Most responses use ISO datetime "2026-06-22T00:00:00".
    if let Some(date_part) = s.split('T').next() {
        if let Ok(d) = NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
            return Some(d);
        }
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

fn interval_from_frequency(freq: &str) -> Option<u32> {
    match freq.trim() {
        "52" => Some(1),
        "26" => Some(2),
        "13" => Some(4),
        // Multi-day-per-week (156) and rare cadences (6, 12) emit
        // explicit dates rather than incorrect RRULE projections.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(cities: Option<&'static [&'static str]>) -> Config {
        Config {
            id: "test",
            name: "Test",
            placeholder: "",
            note: "",
            api_url: "https://example.invalid/api/api/external/",
            cities,
        }
    }

    #[test]
    fn parses_iso_datetime_date() {
        assert_eq!(
            parse_date("2026-06-22T00:00:00"),
            Some(NaiveDate::from_ymd_opt(2026, 6, 22).unwrap())
        );
        assert_eq!(
            parse_date("2026-12-31"),
            Some(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap())
        );
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("garbage"), None);
    }

    #[test]
    fn interval_from_frequency_codes() {
        assert_eq!(interval_from_frequency("52"), Some(1));
        assert_eq!(interval_from_frequency("26"), Some(2));
        assert_eq!(interval_from_frequency("13"), Some(4));
        assert_eq!(interval_from_frequency("156"), None);
        assert_eq!(interval_from_frequency(""), None);
    }

    #[test]
    fn waste_type_label_falls_back() {
        let only_desc = ScheduleEntry {
            date: "2026-01-01T00:00:00".into(),
            type_of_waste_description: "Restavfall".into(),
            waste_type: "REST".into(),
            type_of_waste: "REST".into(),
            title: "...".into(),
            collection_frequency: "52".into(),
        };
        assert_eq!(waste_type_label(&only_desc), "Restavfall");

        let no_desc = ScheduleEntry {
            date: "2026-01-01T00:00:00".into(),
            type_of_waste_description: String::new(),
            waste_type: "REST".into(),
            type_of_waste: "rest".into(),
            title: String::new(),
            collection_frequency: String::new(),
        };
        assert_eq!(waste_type_label(&no_desc), "REST");

        let only_typeof = ScheduleEntry {
            date: "2026-01-01T00:00:00".into(),
            type_of_waste_description: String::new(),
            waste_type: String::new(),
            type_of_waste: "FOOD".into(),
            title: String::new(),
            collection_frequency: String::new(),
        };
        assert_eq!(waste_type_label(&only_typeof), "FOOD");
    }

    #[test]
    fn build_series_groups_and_picks_interval() {
        let entries = vec![
            ScheduleEntry {
                date: "2026-06-22T00:00:00".into(),
                type_of_waste_description: "Restavfall".into(),
                waste_type: "REST".into(),
                type_of_waste: "REST".into(),
                title: "Hämtning av restavfall (kärl 190 liter)".into(),
                collection_frequency: "52".into(),
            },
            ScheduleEntry {
                date: "2026-06-29T00:00:00".into(),
                type_of_waste_description: "Restavfall".into(),
                waste_type: "REST".into(),
                type_of_waste: "REST".into(),
                title: "Hämtning av restavfall (kärl 190 liter)".into(),
                collection_frequency: "52".into(),
            },
            ScheduleEntry {
                date: "2026-07-06T00:00:00".into(),
                type_of_waste_description: "Matavfall".into(),
                waste_type: "FOOD".into(),
                type_of_waste: "FOOD".into(),
                title: "Hämtning av matavfall".into(),
                collection_frequency: "26".into(),
            },
        ];
        let series = build_series(&entries);
        assert_eq!(series.len(), 2);
        let rest = series.iter().find(|s| s.waste_type == "Restavfall").unwrap();
        assert_eq!(rest.interval_weeks, Some(1));
        assert_eq!(rest.anchor.len(), 1);
        assert_eq!(
            rest.anchor[0],
            NaiveDate::from_ymd_opt(2026, 6, 22).unwrap()
        );
        let mat = series.iter().find(|s| s.waste_type == "Matavfall").unwrap();
        assert_eq!(mat.interval_weeks, Some(2));
    }

    #[test]
    fn build_series_keeps_explicit_dates_for_unknown_frequency() {
        let entries = vec![
            ScheduleEntry {
                date: "2026-06-22T00:00:00".into(),
                type_of_waste_description: "Trädgårdsavfall".into(),
                waste_type: "GARDEN".into(),
                type_of_waste: "GARDEN".into(),
                title: "Hämtning av trädgårdsavfall".into(),
                collection_frequency: "12".into(),
            },
            ScheduleEntry {
                date: "2026-07-20T00:00:00".into(),
                type_of_waste_description: "Trädgårdsavfall".into(),
                waste_type: "GARDEN".into(),
                type_of_waste: "GARDEN".into(),
                title: "Hämtning av trädgårdsavfall".into(),
                collection_frequency: "12".into(),
            },
        ];
        let series = build_series(&entries);
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].interval_weeks, None);
        assert_eq!(series[0].anchor.len(), 2);
    }

    #[test]
    fn city_filter_handles_unicode_case() {
        let p = Exde::new(reqwest::Client::new(), cfg(Some(&["SIMRISHAMN", "Ö TOMMARP"])));
        assert!(p.matches_city("simrishamn"));
        assert!(p.matches_city("SIMRISHAMN"));
        assert!(p.matches_city("ö tommarp"));
        assert!(!p.matches_city("TOMELILLA"));
    }

    #[test]
    fn city_filter_none_passes_all() {
        let p = Exde::new(reqwest::Client::new(), cfg(None));
        assert!(p.matches_city("ANYTHING"));
    }
}
