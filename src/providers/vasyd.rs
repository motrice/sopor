//! Provider for VA SYD's open address-lookup API. Covers Malmö and Burlöv
//! kommuner via a shared backend; results are filtered per kommun by the
//! locality label in the returned street string.
//!
//! Endpoints (open POST JSON, no auth):
//!   POST /api/sitecore/mypagesapi/buildingaddresssearch  { "query": "<street>" }
//!     -> { items: [{ street, id }], meta: { success, message } }
//!   POST /api/sitecore/mypagesapi/wastepickupbyaddress   { "query": "<id>", "street": "<full address>" }
//!     -> { items: [{ address, wasteType, wastePickupFrequency, nextWastePickup }], meta: {...} }

use std::collections::{BTreeMap, HashSet};

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::json;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const TYPEAHEAD_URL: &str = "https://www.vasyd.se/api/sitecore/mypagesapi/buildingaddresssearch";
const SEARCH_URL: &str = "https://www.vasyd.se/api/sitecore/mypagesapi/wastepickupbyaddress";

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    /// Localities (city labels) in the kommun. Used to filter results from
    /// the shared VA SYD backend down to a single kommun.
    pub cities: &'static [&'static str],
}

pub struct VaSyd {
    http: reqwest::Client,
    cfg: Config,
}

impl VaSyd {
    pub fn new(http: reqwest::Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    async fn typeahead(&self, query: &str) -> Result<Vec<TypeaheadItem>, ProviderError> {
        let resp: TypeaheadResp = self
            .http
            .post(TYPEAHEAD_URL)
            .header("Content-Type", "application/json")
            .json(&json!({ "query": query }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.items)
    }

    async fn search(&self, id: &str, street: &str) -> Result<Vec<SearchItem>, ProviderError> {
        let resp: SearchResp = self
            .http
            .post(SEARCH_URL)
            .header("Content-Type", "application/json")
            .json(&json!({ "query": id, "street": street }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.items)
    }

    fn matches_city(&self, street: &str) -> bool {
        let city = street.rsplit(',').next().unwrap_or("").trim();
        self.cfg.cities.iter().any(|c| c.eq_ignore_ascii_case(city))
    }
}

#[derive(Deserialize)]
struct TypeaheadResp {
    items: Vec<TypeaheadItem>,
}

#[derive(Deserialize, Debug, Clone)]
struct TypeaheadItem {
    street: String,
    id: String,
}

#[derive(Deserialize)]
struct SearchResp {
    items: Vec<SearchItem>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SearchItem {
    waste_type: String,
    waste_pickup_frequency: String,
    next_waste_pickup: String,
}

#[async_trait]
impl Provider for VaSyd {
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
        let items = self.typeahead(q).await?;
        let mut seen = HashSet::new();
        let mut suggestions = Vec::new();
        for it in items {
            if !self.matches_city(&it.street) {
                continue;
            }
            if seen.insert(it.street.clone()) {
                suggestions.push(Suggestion { value: it.street });
            }
        }
        Ok(suggestions)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let street_part = address.split(',').next().unwrap_or(address).trim();
        let items = self.typeahead(street_part).await?;
        let matching: Vec<TypeaheadItem> = items
            .into_iter()
            .filter(|it| it.street.eq_ignore_ascii_case(address) && self.matches_city(&it.street))
            .collect();

        if matching.is_empty() {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        }

        // (waste_type) -> (frequency_text, set of dates)
        let mut series_map: BTreeMap<String, (String, Vec<NaiveDate>)> = BTreeMap::new();

        for item in matching {
            let entries = self.search(&item.id, address).await?;
            for entry in entries {
                let Ok(date) = NaiveDate::parse_from_str(&entry.next_waste_pickup, "%Y-%m-%d")
                else {
                    continue;
                };
                let bucket = series_map
                    .entry(entry.waste_type)
                    .or_insert_with(|| (entry.waste_pickup_frequency.clone(), Vec::new()));
                if bucket.0.is_empty() {
                    bucket.0 = entry.waste_pickup_frequency;
                }
                bucket.1.push(date);
            }
        }

        let mut series = Vec::new();
        for (waste_type, (frequency_text, mut dates)) in series_map {
            dates.sort();
            dates.dedup();
            let interval_weeks = parse_va_syd_interval(&frequency_text);
            // When we know the weekly cadence, collapse to a single anchor so
            // ical.rs emits a RRULE-projected series; otherwise keep explicit
            // dates and rely on calendar refresh.
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

        Ok(PickupSchedule {
            address: address.to_string(),
            series,
        })
    }
}

fn parse_va_syd_interval(freq: &str) -> Option<u32> {
    let lower = freq.to_lowercase();
    // Single weekday weekly, e.g. "Onsdag varje vecka". Multi-day forms like
    // "Måndag, torsdag varje vecka" are intentionally left without RRULE — a
    // single weekly INTERVAL=1 from a Thursday anchor would miss the Mondays.
    if lower.contains("varje vecka") && !lower.contains(',') {
        return Some(1);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Config {
        Config {
            id: "test",
            name: "Test",
            placeholder: "",
            note: "",
            cities: &["Malmö", "Limhamn"],
        }
    }

    fn provider() -> VaSyd {
        VaSyd::new(reqwest::Client::new(), cfg())
    }

    #[test]
    fn interval_parsing() {
        assert_eq!(parse_va_syd_interval("Onsdag varje vecka"), Some(1));
        assert_eq!(parse_va_syd_interval("Måndag varje vecka"), Some(1));
        // Two weekdays per week — single weekly anchor would miss the other day.
        assert_eq!(parse_va_syd_interval("Måndag, torsdag varje vecka"), None);
        // Absolute week — unclear cadence.
        assert_eq!(parse_va_syd_interval("Måndag Vecka 30"), None);
        assert_eq!(parse_va_syd_interval(""), None);
    }

    #[test]
    fn city_filter_includes_known_localities() {
        let p = provider();
        assert!(p.matches_city("Storgatan 1, Malmö"));
        assert!(p.matches_city("Limhamnsvägen 5, Limhamn"));
        assert!(!p.matches_city("Storgatan 2, Arlöv"));
        assert!(!p.matches_city("Hornsgatan 1, Stockholm"));
    }

    #[test]
    fn typeahead_response_deserializes() {
        let json = r#"{"query":"Storgatan","items":[
            {"street":"Storgatan 1, Malmö","id":"185904"},
            {"street":"Storgatan 2, Arlöv","id":"131172"}
        ],"meta":{"success":true,"message":null}}"#;
        let resp: TypeaheadResp = serde_json::from_str(json).unwrap();
        assert_eq!(resp.items.len(), 2);
        assert_eq!(resp.items[0].id, "185904");
        assert_eq!(resp.items[1].street, "Storgatan 2, Arlöv");
    }

    #[test]
    fn search_response_deserializes() {
        let json = r#"{"items":[
            {"address":"Storgatan 1, Malmö","wasteType":"Restavfall",
             "wastePickupFrequency":"Måndag, torsdag varje vecka",
             "nextWastePickup":"2026-06-18"},
            {"address":"Storgatan 1, Malmö","wasteType":"Glasförp.färg",
             "wastePickupFrequency":"Måndag Vecka 30",
             "nextWastePickup":"2026-07-20"}
        ],"meta":{"success":true,"message":null}}"#;
        let resp: SearchResp = serde_json::from_str(json).unwrap();
        assert_eq!(resp.items.len(), 2);
        assert_eq!(resp.items[0].waste_type, "Restavfall");
        assert_eq!(resp.items[1].next_waste_pickup, "2026-07-20");
    }
}
