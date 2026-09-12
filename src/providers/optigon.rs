//! Optigon Avfallskollen — publikt REST-API på
//! `avfallskollen-api.optigon.se` som backar SPAn på
//! avfallskollen.optigon.se. Två endpoints, båda anonyma:
//!
//!   GET /locations?search=<query>
//!     → [{ Id: "<uuid>", Address: "STREET N, CITY" }]  (max 20 träffar)
//!   GET /pickup-events/<uuid>
//!     → { LocationId, Address, PickupEvents: [{ Date, Fractions: [{ Id, Name }] }] }
//!
//! PickupEvents täcker hela året framåt med fraktioner per hämtdag.
//! Vi grupperar efter fraktion och emitterar explicita datum utan
//! RRULE.

use async_trait::async_trait;
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;
use std::collections::BTreeMap;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const BASE: &str = "https://avfallskollen-api.optigon.se";

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    pub cities: &'static [&'static str],
}

pub struct Optigon {
    http: Client,
    cfg: Config,
}

impl Optigon {
    pub fn new(http: Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    fn matches_city(&self, city: &str) -> bool {
        let needle = city.trim().to_lowercase();
        self.cfg.cities.iter().any(|c| c.to_lowercase() == needle)
    }
}

#[derive(Debug, Deserialize)]
struct Location {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Address")]
    address: String,
}

#[derive(Debug, Deserialize)]
struct PickupResponse {
    #[serde(rename = "Address")]
    address: Option<String>,
    #[serde(rename = "PickupEvents", default)]
    pickup_events: Vec<PickupEvent>,
}

#[derive(Debug, Deserialize)]
struct PickupEvent {
    #[serde(rename = "Date")]
    date: String,
    #[serde(rename = "Fractions", default)]
    fractions: Vec<Fraction>,
}

#[derive(Debug, Deserialize)]
struct Fraction {
    #[serde(rename = "Name")]
    name: String,
}

#[async_trait]
impl Provider for Optigon {
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
        let locations: Vec<Location> = self
            .http
            .get(format!("{BASE}/locations"))
            .query(&[("search", q)])
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;

        Ok(locations
            .into_iter()
            .filter(|l| self.matches_city(city_of(&l.address)))
            .map(|l| Suggestion {
                value: format!("{} (id={})", l.address, l.id),
            })
            .collect())
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let Some(id) = extract_id(address) else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };
        let resp: PickupResponse = self
            .http
            .get(format!("{BASE}/pickup-events/{id}"))
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;

        Ok(build_schedule(
            resp.address.unwrap_or_else(|| address.to_string()),
            resp.pickup_events,
        ))
    }
}

fn city_of(s: &str) -> &str {
    s.rsplit_once(',')
        .map(|(_, c)| c.trim())
        .unwrap_or(s)
}

fn extract_id(s: &str) -> Option<String> {
    let open = s.rfind('(')?;
    let close = s.rfind(')')?;
    if close <= open {
        return None;
    }
    let inner = &s[open + 1..close];
    inner.strip_prefix("id=").map(|v| v.to_string())
}

fn build_schedule(address: String, events: Vec<PickupEvent>) -> PickupSchedule {
    let mut by_fraction: BTreeMap<String, Vec<NaiveDate>> = BTreeMap::new();
    for e in events {
        let Some(date) = NaiveDate::parse_from_str(&e.date, "%Y-%m-%d").ok() else {
            continue;
        };
        for f in e.fractions {
            let list = by_fraction.entry(f.name).or_default();
            if !list.contains(&date) {
                list.push(date);
            }
        }
    }
    let series = by_fraction
        .into_iter()
        .map(|(waste_type, mut anchor)| {
            anchor.sort();
            PickupSeries {
                waste_type,
                frequency_text: "Optigon Avfallskollen".to_string(),
                interval_weeks: None,
                anchor,
            }
        })
        .collect();
    PickupSchedule { address, series }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn city_of_extracts_trailing() {
        assert_eq!(city_of("Storgatan 1, Forshaga"), "Forshaga");
        assert_eq!(city_of("Mörmovägen 4, Hammarö"), "Hammarö");
        assert_eq!(city_of("NoComma"), "NoComma");
    }

    #[test]
    fn extract_id_from_suggestion() {
        assert_eq!(
            extract_id("Storgatan 1, Forshaga (id=1E1BBCD2-D9F2-443E-AA22-5DA8CCA5C462)"),
            Some("1E1BBCD2-D9F2-443E-AA22-5DA8CCA5C462".into())
        );
        assert!(extract_id("Storgatan 1, Forshaga").is_none());
        assert!(extract_id("(id-broken").is_none());
    }

    #[test]
    fn build_schedule_groups_fractions() {
        let events = vec![
            PickupEvent {
                date: "2026-09-18".into(),
                fractions: vec![
                    Fraction { name: "Matavfall".into() },
                    Fraction { name: "Restavfall".into() },
                ],
            },
            PickupEvent {
                date: "2026-10-02".into(),
                fractions: vec![
                    Fraction { name: "Matavfall".into() },
                    Fraction { name: "Restavfall".into() },
                ],
            },
        ];
        let s = build_schedule("Storgatan 1, Forshaga".into(), events);
        assert_eq!(s.series.len(), 2);
        let m = s.series.iter().find(|p| p.waste_type == "Matavfall").unwrap();
        assert_eq!(m.anchor.len(), 2);
    }
}
