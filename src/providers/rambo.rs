//! Rambo AB — Lysekil, Munkedal, Sotenäs, Tanum. Anonymt WP-JSON-API
//! på `rambo.se/wp-json/app/v1/` med statisk `X-App-Identifier`-header
//! som extraherats ur den publika pickup-widgetens JS-bundle
//! (`wp-content/themes/rambo/assets/js/pickup-new.js`).
//!
//! Två endpoints används:
//!   GET /wp-json/app/v1/address-flat?address=<query>
//!     → array; poster med `plant_number` är byggnader, poster med
//!       `is_key: true` är alfabetiska bokstavsrubriker som filtreras
//!       bort.
//!   GET /wp-json/app/v1/next-pickup-web?plant-number=<plant_number>
//!     → { types: [{ type, pickup_date, ... }], address, city }
//!
//! API:t returnerar bara *nästa* tömning per fraktion (som Stockholm),
//! så vi emitterar en explicit anchor-datum-serie per fraktion utan
//! RRULE och förlitar oss på klientens 12 h refresh.

use async_trait::async_trait;
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const APP_IDENTIFIER: &str = "202d6cd8-389c-4ab4-8921-7183378eb477";
const BASE: &str = "https://rambo.se/wp-json/app/v1";

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    pub cities: &'static [&'static str],
}

pub struct Rambo {
    http: Client,
    cfg: Config,
}

impl Rambo {
    pub fn new(http: Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    fn matches_city(&self, city: &str) -> bool {
        let needle = city.trim().to_lowercase();
        self.cfg.cities.iter().any(|c| c.to_lowercase() == needle)
    }
}

#[derive(Debug, Deserialize)]
struct AddressEntry {
    plant_number: Option<String>,
    address: Option<String>,
    zip_city: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NextPickup {
    types: Vec<PickupType>,
    #[serde(default)]
    address: Option<String>,
    #[serde(default)]
    city: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PickupType {
    #[serde(rename = "type")]
    kind: String,
    pickup_date: String,
}

#[async_trait]
impl Provider for Rambo {
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
        let url = format!("{BASE}/address-flat");
        let entries: Vec<AddressEntry> = self
            .http
            .get(&url)
            .query(&[("address", q)])
            .header("X-App-Identifier", APP_IDENTIFIER)
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;

        Ok(entries
            .into_iter()
            .filter_map(|e| suggestion_from(e, |c| self.matches_city(c)))
            .collect())
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let Some(pn) = extract_plant_number(address) else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };
        let url = format!("{BASE}/next-pickup-web");
        let next: NextPickup = self
            .http
            .get(&url)
            .query(&[("plant-number", pn.as_str())])
            .header("X-App-Identifier", APP_IDENTIFIER)
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;

        Ok(build_schedule(address.to_string(), next))
    }
}

fn suggestion_from(e: AddressEntry, city_ok: impl Fn(&str) -> bool) -> Option<Suggestion> {
    let pn = e.plant_number?;
    let addr = e.address?;
    let city = e.zip_city.unwrap_or_default();
    if city.is_empty() || !city_ok(&city) {
        return None;
    }
    // Upstream's `address` field already ends with ", <city>", so avoid
    // duplicating the city.
    let display = if addr.trim_end().ends_with(&format!(", {city}")) {
        addr
    } else {
        format!("{addr}, {city}")
    };
    Some(Suggestion {
        value: format!("{display} (pn={pn})"),
    })
}

/// Extract `pn=<value>` from the last `(...)` group in the suggestion string.
fn extract_plant_number(s: &str) -> Option<String> {
    let open = s.rfind('(')?;
    let close = s.rfind(')')?;
    if close < open {
        return None;
    }
    let inner = &s[open + 1..close];
    inner.strip_prefix("pn=").map(|v| v.to_string())
}

fn build_schedule(address: String, next: NextPickup) -> PickupSchedule {
    let series = next
        .types
        .into_iter()
        .filter_map(|t| {
            let date = NaiveDate::parse_from_str(&t.pickup_date, "%Y-%m-%d").ok()?;
            Some(PickupSeries {
                waste_type: t.kind,
                frequency_text: "Nästa tömning (visas ~14 dagar i förväg)".to_string(),
                interval_weeks: None,
                anchor: vec![date],
            })
        })
        .collect();
    let display = match (next.address, next.city) {
        (Some(a), Some(c)) => format!("{a}, {c}"),
        (Some(a), None) => a,
        _ => address,
    };
    PickupSchedule {
        address: display,
        series,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_plant_number_handles_base64_with_special_chars() {
        assert_eq!(
            extract_plant_number("Storgatan 1, Grebbestad (pn=k2nirOKR28S0wZPHKWo6LgkZiWtW+Nu9Wq8i6eO/bhE=)"),
            Some("k2nirOKR28S0wZPHKWo6LgkZiWtW+Nu9Wq8i6eO/bhE=".to_string())
        );
    }

    #[test]
    fn extract_plant_number_returns_none_when_missing() {
        assert!(extract_plant_number("Storgatan 1, Grebbestad").is_none());
        assert!(extract_plant_number("Storgatan 1, Grebbestad (foo)").is_none());
    }

    #[test]
    fn suggestion_filters_out_key_entries() {
        // Entries with is_key: true have no plant_number → filtered out.
        let key = AddressEntry {
            plant_number: None,
            address: None,
            zip_city: None,
        };
        assert!(suggestion_from(key, |_| true).is_none());
    }

    #[test]
    fn suggestion_filters_by_city_allow_list() {
        let e = AddressEntry {
            plant_number: Some("abc==".into()),
            address: Some("Storgatan 1".into()),
            zip_city: Some("Kungshamn".into()),
        };
        let ok = suggestion_from(e, |c| c == "Kungshamn");
        assert!(ok.is_some());
        assert!(ok.unwrap().value.contains("(pn=abc==)"));

        let e2 = AddressEntry {
            plant_number: Some("xyz".into()),
            address: Some("Storgatan 1".into()),
            zip_city: Some("Grebbestad".into()),
        };
        assert!(suggestion_from(e2, |c| c == "Kungshamn").is_none());
    }

    #[test]
    fn build_schedule_emits_one_series_per_type() {
        let next = NextPickup {
            types: vec![
                PickupType {
                    kind: "Kärl 1".into(),
                    pickup_date: "2026-09-18".into(),
                },
                PickupType {
                    kind: "Kärl 2".into(),
                    pickup_date: "2026-09-23".into(),
                },
            ],
            address: Some("Storgatan 1".into()),
            city: Some("Grebbestad".into()),
        };
        let s = build_schedule("input".into(), next);
        assert_eq!(s.address, "Storgatan 1, Grebbestad");
        assert_eq!(s.series.len(), 2);
        assert_eq!(s.series[0].waste_type, "Kärl 1");
        assert_eq!(
            s.series[0].anchor,
            vec![NaiveDate::from_ymd_opt(2026, 9, 18).unwrap()]
        );
        assert!(s.series[0].interval_weeks.is_none());
    }

    #[test]
    fn build_schedule_skips_unparseable_dates() {
        let next = NextPickup {
            types: vec![PickupType {
                kind: "Kärl 1".into(),
                pickup_date: "invalid".into(),
            }],
            address: None,
            city: None,
        };
        let s = build_schedule("fallback".into(), next);
        assert!(s.series.is_empty());
        assert_eq!(s.address, "fallback");
    }
}
