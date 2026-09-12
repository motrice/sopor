//! Renhållningen Kristianstad — anonymt Appbolaget-universal-API.
//! Widgeten på `renhallningen-kristianstad.se/hushall/villa-fritidshus/
//! tomning-av-sopkarl/tomningschema/` renderas av en Vue-komponent i
//! `utility-bar.js` som anropar två endpoints med `Unit`-header:
//!
//!   GET https://api-universal.appbolaget.se/waste/addresses/search?query=<q>&limit=50
//!     → [{ uuid, address, city, ... }]
//!   GET https://api-universal.appbolaget.se/waste/addresses/<uuid>
//!     → { services: [{ type, description, collection_at, precision }] }
//!
//! Notera skillnaden mot vår befintliga `hassleholm.rs`: Hässleholms
//! Appbolaget-tenant returnerar bara sortering/id, medan Kristianstads
//! direkt returnerar `services` med `collection_at` — den *nästa*
//! hämtningen per fraktion. Vi emitterar explicita anchor-datum utan
//! RRULE och förlitar oss på klientens 12 h refresh.

use async_trait::async_trait;
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const BASE: &str = "https://api-universal.appbolaget.se/waste/addresses";
const SOURCE: &str = "renhallningen-kristianstad.se";
const UNIT: &str = "dd905ce7-b16d-4422-be36-564169af4035";

pub struct Kristianstad {
    http: Client,
}

impl Kristianstad {
    pub fn new(http: Client) -> Self {
        Self { http }
    }

    fn get(&self, url: String) -> reqwest::RequestBuilder {
        self.http
            .get(url)
            .header("Module", "universal")
            .header("Source", SOURCE)
            .header("Unit", UNIT)
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    data: Vec<SearchHit>,
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    uuid: String,
    address: Option<String>,
    city: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddressResponse {
    data: AddressData,
}

#[derive(Debug, Deserialize)]
struct AddressData {
    #[serde(default)]
    address: Option<String>,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    services: Vec<Service>,
}

#[derive(Debug, Deserialize)]
struct Service {
    #[serde(default, rename = "type")]
    fraction: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    collection_at: Option<String>,
}

#[async_trait]
impl Provider for Kristianstad {
    fn id(&self) -> &'static str {
        "kristianstad"
    }
    fn name(&self) -> &'static str {
        "Kristianstad"
    }
    fn placeholder(&self) -> &'static str {
        "t.ex. Västra Storgatan 1"
    }
    fn note(&self) -> &'static str {
        "Sophämtningsdata från Renhållningen Kristianstad via \
         Appbolaget-universal-API. API:t returnerar bara nästa tömning \
         per fraktion — kalendern uppdateras löpande när klienten \
         hämtar in feeden på nytt."
    }

    async fn autocomplete(&self, query: &str) -> Result<Vec<Suggestion>, ProviderError> {
        let q = query.trim();
        if q.len() < 2 {
            return Ok(vec![]);
        }
        let url = format!("{BASE}/search?query={}&limit=50", urlencode(q));
        let resp: SearchResponse = self
            .get(url)
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;

        Ok(resp
            .data
            .into_iter()
            .filter_map(|h| suggestion_from(h))
            .collect())
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let Some(uuid) = extract_uuid(address) else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };
        let url = format!("{BASE}/{uuid}");
        let resp: AddressResponse = self
            .get(url)
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;

        Ok(build_schedule(address.to_string(), resp.data))
    }
}

fn suggestion_from(h: SearchHit) -> Option<Suggestion> {
    let addr = h.address?;
    let city = h.city.unwrap_or_default();
    let display = if city.is_empty() {
        addr
    } else if addr.trim_end().ends_with(&format!(", {city}")) {
        addr
    } else {
        format!("{addr}, {city}")
    };
    Some(Suggestion {
        value: format!("{display} (id={})", h.uuid),
    })
}

fn extract_uuid(s: &str) -> Option<String> {
    let open = s.rfind('(')?;
    let close = s.rfind(')')?;
    if close <= open {
        return None;
    }
    let inner = &s[open + 1..close];
    inner.strip_prefix("id=").map(|v| v.to_string())
}

fn urlencode(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}

fn build_schedule(fallback: String, data: AddressData) -> PickupSchedule {
    let display = match (data.address, data.city) {
        (Some(a), Some(c)) if !a.trim_end().ends_with(&format!(", {c}")) => format!("{a}, {c}"),
        (Some(a), _) => a,
        _ => fallback,
    };
    let series = data
        .services
        .into_iter()
        .filter_map(|s| {
            let date = NaiveDate::parse_from_str(s.collection_at.as_deref()?, "%Y-%m-%d").ok()?;
            let waste_type = s.fraction.unwrap_or_else(|| "Hämtning".to_string());
            let desc = s
                .description
                .unwrap_or_else(|| "Nästa hämtning".to_string());
            Some(PickupSeries {
                waste_type,
                frequency_text: desc,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_uuid_from_suggestion() {
        assert_eq!(
            extract_uuid("Västra Storgatan 1, Kristianstad (id=1ec2efb9-115c-4c52-9274-74398e9f427f)"),
            Some("1ec2efb9-115c-4c52-9274-74398e9f427f".to_string())
        );
        assert!(extract_uuid("Västra Storgatan 1").is_none());
    }

    #[test]
    fn suggestion_appends_uuid() {
        let h = SearchHit {
            uuid: "abc-123".into(),
            address: Some("Storgatan 1".into()),
            city: Some("Kristianstad".into()),
        };
        assert_eq!(
            suggestion_from(h).unwrap().value,
            "Storgatan 1, Kristianstad (id=abc-123)"
        );
    }

    #[test]
    fn suggestion_avoids_duplicate_city() {
        let h = SearchHit {
            uuid: "abc-123".into(),
            address: Some("Storgatan 1, Kristianstad".into()),
            city: Some("Kristianstad".into()),
        };
        assert_eq!(
            suggestion_from(h).unwrap().value,
            "Storgatan 1, Kristianstad (id=abc-123)"
        );
    }

    #[test]
    fn build_schedule_emits_one_series_per_service() {
        let data = AddressData {
            address: Some("Västra Storgatan".into()),
            city: Some("Kristianstad".into()),
            services: vec![
                Service {
                    fraction: Some("Matavfall".into()),
                    description: Some("Matavfall i underjordsbehållare, varannan vecka".into()),
                    collection_at: Some("2026-09-23".into()),
                },
                Service {
                    fraction: Some("Restavfall".into()),
                    description: Some("Restavfall i underjordsbehållare, varannan vecka 3 kbm".into()),
                    collection_at: Some("2026-09-23".into()),
                },
                Service {
                    fraction: Some("Pappersförp".into()),
                    description: Some("Pappersförp i underjordsbehållare, var 4:e vecka 5 kbm".into()),
                    collection_at: Some("2026-10-07".into()),
                },
            ],
        };
        let s = build_schedule("fallback".into(), data);
        assert_eq!(s.address, "Västra Storgatan, Kristianstad");
        assert_eq!(s.series.len(), 3);
        assert_eq!(s.series[0].waste_type, "Matavfall");
        assert_eq!(
            s.series[0].anchor,
            vec![NaiveDate::from_ymd_opt(2026, 9, 23).unwrap()]
        );
    }

    #[test]
    fn build_schedule_skips_missing_dates() {
        let data = AddressData {
            address: None,
            city: None,
            services: vec![Service {
                fraction: Some("X".into()),
                description: None,
                collection_at: None,
            }],
        };
        let s = build_schedule("fallback".into(), data);
        assert!(s.series.is_empty());
        assert_eq!(s.address, "fallback");
    }
}
