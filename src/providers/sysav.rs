//! Sysav — publik EDP-proxy för Sysav-ägarkommuner. Widgeten på
//! `sysav.se/privat/min-sophamtning/<kommun>/` läser från en anonym
//! .NET-tjänst deployad som Azure Container App:
//!
//!   GET {BASE}/PickupSchedules/findbuilding/<query>
//!     → JSON-array av "STREET N, CITY"-strängar
//!   GET {BASE}/PickupSchedules/foraddress/<address>
//!     → array av { nextPickupDate, wasteType, pickupFrequency, ... }
//!
//! API:t returnerar bara *nästa* tömning per fraktion (som Stockholm/
//! Rambo), så vi emitterar explicit anchor-datum utan RRULE och
//! förlitar oss på klientens 12 h refresh.

use async_trait::async_trait;
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const BASE: &str = "https://ca-swec-sysav-public-edp-prod.bluedune-a5ae63ed.swedencentral.azurecontainerapps.io/api";

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    pub cities: &'static [&'static str],
}

pub struct Sysav {
    http: Client,
    cfg: Config,
}

impl Sysav {
    pub fn new(http: Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    fn matches_city(&self, city: &str) -> bool {
        let needle = city.trim().to_lowercase();
        self.cfg.cities.iter().any(|c| c.to_lowercase() == needle)
    }
}

#[derive(Debug, Deserialize)]
struct ScheduleEntry {
    #[serde(rename = "nextPickupDate")]
    next_pickup_date: Option<String>,
    #[serde(rename = "wasteType")]
    waste_type: Option<String>,
    #[serde(rename = "pickupFrequency")]
    pickup_frequency: Option<String>,
    #[serde(rename = "binSize")]
    bin_size: Option<String>,
    #[serde(rename = "binUnit")]
    bin_unit: Option<String>,
    #[serde(rename = "binType")]
    bin_type: Option<String>,
    #[serde(rename = "address")]
    address: Option<String>,
}

#[async_trait]
impl Provider for Sysav {
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
        let url = format!("{BASE}/PickupSchedules/findbuilding/{}", urlencode(q));
        let raw: Vec<String> = self
            .http
            .get(&url)
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
        let url = format!("{BASE}/PickupSchedules/foraddress/{}", urlencode(address));
        let entries: Vec<ScheduleEntry> = self
            .http
            .get(&url)
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

fn urlencode(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}

fn build_schedule(fallback_address: String, entries: Vec<ScheduleEntry>) -> PickupSchedule {
    let display = entries
        .first()
        .and_then(|e| e.address.clone())
        .unwrap_or(fallback_address);

    // Slå ihop poster med samma waste_type + nextPickupDate (Svedala har t.ex.
    // två byggnad-IDs per adress som ger dubletter).
    let mut seen = std::collections::HashSet::new();
    let series = entries
        .into_iter()
        .filter_map(|e| {
            let date_str = e.next_pickup_date?;
            let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok()?;
            let waste_type = e.waste_type?;
            if !seen.insert((waste_type.clone(), date)) {
                return None;
            }
            let freq = describe_frequency(
                e.pickup_frequency.as_deref(),
                e.bin_size.as_deref(),
                e.bin_unit.as_deref(),
                e.bin_type.as_deref(),
            );
            Some(PickupSeries {
                waste_type,
                frequency_text: freq,
                interval_weeks: None,
                anchor: vec![date],
            })
        })
        .collect();
    PickupSchedule {
        address: display,
        series,
    }
}

fn describe_frequency(
    weekday: Option<&str>,
    size: Option<&str>,
    unit: Option<&str>,
    bin_type: Option<&str>,
) -> String {
    let mut parts = Vec::new();
    if let Some(w) = weekday {
        let w = w.trim();
        if !w.is_empty() {
            let mut cap = w.chars();
            let head = cap
                .next()
                .map(|c| c.to_uppercase().collect::<String>())
                .unwrap_or_default();
            parts.push(format!("{}{}", head, cap.as_str()));
        }
    }
    let bin = match (size, unit, bin_type) {
        (Some(s), Some(u), Some(t)) => format!("{} {} {}", s.trim(), u.trim(), t.trim().to_lowercase()),
        (Some(s), Some(u), None) => format!("{} {}", s.trim(), u.trim()),
        (_, _, Some(t)) => t.trim().to_lowercase(),
        _ => String::new(),
    };
    if !bin.is_empty() {
        parts.push(bin);
    }
    if parts.is_empty() {
        "Nästa tömning".to_string()
    } else {
        parts.join(" — ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn city_of_extracts_last_segment() {
        assert_eq!(city_of("Storgatan 12, Kävlinge"), "Kävlinge");
        assert_eq!(city_of("STORGATAN 14, APOTEK, SIMRISHAMN"), "SIMRISHAMN");
        assert_eq!(city_of("NoComma"), "NoComma");
    }

    #[test]
    fn build_schedule_dedups_same_type_and_date() {
        let entries = vec![
            ScheduleEntry {
                next_pickup_date: Some("2026-09-14".into()),
                waste_type: Some("Restavfall".into()),
                pickup_frequency: Some("måndag".into()),
                bin_size: Some("190,00".into()),
                bin_unit: Some("l".into()),
                bin_type: Some("Kärl".into()),
                address: Some("Storgatan 10, Svedala".into()),
            },
            ScheduleEntry {
                next_pickup_date: Some("2026-09-14".into()),
                waste_type: Some("Restavfall".into()),
                pickup_frequency: Some("måndag".into()),
                bin_size: Some("660,00".into()),
                bin_unit: Some("l".into()),
                bin_type: Some("Kärl".into()),
                address: Some("Storgatan 10, Svedala".into()),
            },
        ];
        let s = build_schedule("input".into(), entries);
        assert_eq!(s.series.len(), 1);
        assert_eq!(s.series[0].waste_type, "Restavfall");
        assert_eq!(s.address, "Storgatan 10, Svedala");
    }

    #[test]
    fn build_schedule_emits_multiple_types() {
        let entries = vec![
            ScheduleEntry {
                next_pickup_date: Some("2026-09-25".into()),
                waste_type: Some("Kärl 1".into()),
                pickup_frequency: Some("fredag".into()),
                bin_size: None, bin_unit: None, bin_type: None,
                address: Some("Storgatan 12, Kävlinge".into()),
            },
            ScheduleEntry {
                next_pickup_date: Some("2026-10-02".into()),
                waste_type: Some("Kärl 2".into()),
                pickup_frequency: Some("fredag".into()),
                bin_size: None, bin_unit: None, bin_type: None,
                address: Some("Storgatan 12, Kävlinge".into()),
            },
        ];
        let s = build_schedule("input".into(), entries);
        assert_eq!(s.series.len(), 2);
        assert!(s.series[0].frequency_text.contains("Fredag"));
    }

    #[test]
    fn describe_frequency_variants() {
        assert_eq!(
            describe_frequency(Some("måndag"), Some("190,00"), Some("l"), Some("Kärl")),
            "Måndag — 190,00 l kärl"
        );
        assert_eq!(describe_frequency(Some("fredag"), None, None, None), "Fredag");
        assert_eq!(describe_frequency(None, None, None, None), "Nästa tömning");
    }
}
