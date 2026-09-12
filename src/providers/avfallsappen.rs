//! Provider for kommuner served by an Avfallsappen (Bozzanova) embedded
//! widget. The mobile app itself requires a bearer + device-registration
//! dance, but Bozzanova ships a Vue widget for embedding on kommunsajter
//! that side-steps device registration by hardcoding a static bearer and
//! `X-App-Identifier` in the shipped JavaScript.
//!
//! First tenant wired up: `gullspang.avfallsapp.se`, which serves all 13
//! member kommuner of Avfall & Återvinning Skaraborg (AÅS) via the widget
//! embedded on `avfallskaraborg.se/sophamtning/se-tomningsdagar-…/#/`.
//!
//! Endpoints:
//!   GET  https://<tenant>.avfallsapp.se/api/nova/v1/next-pickup/search?address=<street>
//!     -> { "<Ort>": [{address, zip_city, plant_number, type}, ...], ... }
//!   POST https://<tenant>.avfallsapp.se/api/nova/v1/next-pickup/address?plant_id=<plant_number>
//!     -> { address, city, plant_id, bins: [{type, pickup_date, formatted}, ...] }
//!
//! Required headers on both calls:
//!   Authorization: Bearer <static token from widget JS>
//!   X-App-Identifier: <static UUID from widget JS>
//!
//! The `plant_number` field returned by search is a Laravel-encrypted
//! opaque blob (~360 bytes). We do NOT expose it in autocomplete values
//! — instead we present "<street>, <ort>" to the user and re-search on
//! schedule() to resolve back to the plant_number, using the ort segment
//! to disambiguate cross-postort collisions.
//!
//! Response format is "next pickup only" per bin type (no RRULE, no
//! multi-month calendar). Emit as explicit single-date anchors and rely
//! on iCal client refresh (~12 h) to pull in the following pickup once
//! the current one passes.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Deserialize;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    /// Subdomain of avfallsapp.se hosting the embedded-widget endpoints.
    pub tenant: &'static str,
    /// Static bearer copied out of the widget JS.
    pub bearer: &'static str,
    /// Static X-App-Identifier UUID copied out of the widget JS.
    pub app_identifier: &'static str,
    /// Allow-list of `zip_city` values that belong to this kommun. Matched
    /// case-insensitively (Unicode). Cross-kommun entries returned by the
    /// shared tenant are dropped.
    pub cities: &'static [&'static str],
}

pub struct Avfallsappen {
    http: reqwest::Client,
    cfg: Config,
}

impl Avfallsappen {
    pub fn new(http: reqwest::Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    fn base(&self) -> String {
        format!("https://{}.avfallsapp.se/api/nova/v1/next-pickup", self.cfg.tenant)
    }

    async fn search(&self, query: &str) -> Result<Vec<SearchHit>, ProviderError> {
        let url = format!("{}/search", self.base());
        let resp = self
            .http
            .get(&url)
            .query(&[("address", query)])
            .header("Authorization", format!("Bearer {}", self.cfg.bearer))
            .header("X-App-Identifier", self.cfg.app_identifier)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;
        let text = self.check_auth(resp).await?.text().await?;
        parse_search(&text)
    }

    async fn fetch_pickup(&self, plant_number: &str) -> Result<String, ProviderError> {
        let url = format!("{}/address", self.base());
        let resp = self
            .http
            .post(&url)
            .query(&[("plant_id", plant_number)])
            .header("Authorization", format!("Bearer {}", self.cfg.bearer))
            .header("X-App-Identifier", self.cfg.app_identifier)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::CONTENT_LENGTH, "0")
            .body("")
            .send()
            .await?;
        let text = self.check_auth(resp).await?.text().await?;
        Ok(text)
    }

    // Emit a loud, one-line hint when upstream rejects our static bearer.
    // Wayback shows the widget has shipped the same token for ~19 months, so
    // a 401 almost certainly means Bozzanova rotated the widget key — the
    // fix is to lift a fresh Bearer + X-App-Identifier from the current JS
    // bundle on avfallskaraborg.se and update the two strings in Registry.
    async fn check_auth(&self, resp: reqwest::Response) -> Result<reqwest::Response, ProviderError> {
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            tracing::warn!(
                target: "avfallsappen",
                provider = self.cfg.id,
                tenant = self.cfg.tenant,
                "upstream 401 — widget bearer/X-App-Identifier likely rotated; \
                 refresh from the current widget JS on the kommunsajt"
            );
        }
        Ok(resp.error_for_status()?)
    }

    fn matches_city(&self, city: &str) -> bool {
        let needle = city.to_lowercase();
        self.cfg
            .cities
            .iter()
            .any(|c| c.to_lowercase() == needle)
    }
}

#[derive(Debug, Clone)]
struct SearchHit {
    address: String,
    zip_city: String,
    plant_number: String,
}

fn parse_search(body: &str) -> Result<Vec<SearchHit>, ProviderError> {
    // Response shape is a map from postort -> array of hits. An empty match
    // is emitted as `[]` at the top level, so accept both.
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Resp {
        Map(BTreeMap<String, Vec<RawHit>>),
        // Upstream returns a bare `[]` when there are no matches (rather
        // than an empty object). Accept both shapes.
        #[allow(dead_code)]
        Empty(Vec<serde::de::IgnoredAny>),
    }
    #[derive(Deserialize)]
    struct RawHit {
        #[serde(default)]
        address: String,
        #[serde(default)]
        zip_city: String,
        #[serde(default)]
        plant_number: String,
    }
    let parsed: Resp = serde_json::from_str(body)?;
    let mut out = Vec::new();
    if let Resp::Map(m) = parsed {
        for (_bucket, hits) in m {
            for h in hits {
                if h.address.is_empty() || h.plant_number.is_empty() {
                    continue;
                }
                out.push(SearchHit {
                    address: h.address,
                    zip_city: h.zip_city,
                    plant_number: h.plant_number,
                });
            }
        }
    }
    Ok(out)
}

#[derive(Deserialize)]
struct PickupResp {
    #[serde(default)]
    address: String,
    #[serde(default)]
    city: String,
    #[serde(default)]
    bins: Vec<Bin>,
}

#[derive(Deserialize)]
struct Bin {
    #[serde(default, rename = "type")]
    ty: String,
    #[serde(default)]
    pickup_date: String,
}

fn parse_pickup(body: &str) -> Result<(String, Vec<PickupSeries>), ProviderError> {
    let resp: PickupResp = serde_json::from_str(body)?;
    let address = match (resp.address.trim(), resp.city.trim()) {
        ("", "") => String::new(),
        (a, "") => a.to_string(),
        ("", c) => c.to_string(),
        (a, c) => format!("{a}, {c}"),
    };

    // Group by waste type, dedup dates (same fraction sometimes appears
    // multiple times on the same day; seen on Götene test address).
    let mut by_type: BTreeMap<String, Vec<NaiveDate>> = BTreeMap::new();
    for bin in resp.bins {
        if bin.ty.is_empty() {
            continue;
        }
        let Ok(date) = NaiveDate::parse_from_str(&bin.pickup_date, "%Y-%m-%d") else {
            continue;
        };
        by_type.entry(bin.ty).or_default().push(date);
    }

    let series = by_type
        .into_iter()
        .map(|(waste_type, mut dates)| {
            dates.sort();
            dates.dedup();
            PickupSeries {
                waste_type,
                frequency_text: String::new(),
                interval_weeks: None,
                anchor: dates,
            }
        })
        .collect();
    Ok((address, series))
}

fn split_input(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    match trimmed.split_once(',') {
        Some((street, city)) => (street.trim().to_string(), {
            let c = city.trim();
            if c.is_empty() { None } else { Some(c.to_string()) }
        }),
        None => (trimmed.to_string(), None),
    }
}

#[async_trait]
impl Provider for Avfallsappen {
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
        let hits = self.search(q).await?;
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for hit in hits {
            if !self.matches_city(&hit.zip_city) {
                continue;
            }
            let value = if hit.zip_city.is_empty() {
                hit.address
            } else {
                format!("{}, {}", hit.address, hit.zip_city)
            };
            if seen.insert(value.clone()) {
                out.push(Suggestion { value });
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let (street, city_hint) = split_input(address);
        if street.is_empty() {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        }
        let hits = self.search(&street).await?;

        // Prefer an exact street+city match, then street-only within the
        // kommun's allow-list. Fall back to the first allow-listed hit.
        let matched = hits
            .iter()
            .find(|h| {
                self.matches_city(&h.zip_city)
                    && h.address.eq_ignore_ascii_case(&street)
                    && city_hint
                        .as_deref()
                        .map(|c| h.zip_city.to_lowercase() == c.to_lowercase())
                        .unwrap_or(true)
            })
            .or_else(|| {
                hits.iter().find(|h| {
                    self.matches_city(&h.zip_city) && h.address.eq_ignore_ascii_case(&street)
                })
            })
            .or_else(|| hits.iter().find(|h| self.matches_city(&h.zip_city)));

        let Some(hit) = matched else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };

        let body = self.fetch_pickup(&hit.plant_number).await?;
        let (resolved_address, series) = parse_pickup(&body)?;
        let out_address = if resolved_address.is_empty() {
            format!("{}, {}", hit.address, hit.zip_city)
        } else {
            resolved_address
        };
        Ok(PickupSchedule {
            address: out_address,
            series,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(cities: &'static [&'static str]) -> Config {
        Config {
            id: "test",
            name: "Test",
            placeholder: "",
            note: "",
            tenant: "example",
            bearer: "token",
            app_identifier: "uuid",
            cities,
        }
    }

    #[test]
    fn parse_search_groups_and_flattens() {
        let body = r#"{
            "Falköping":[
                {"address":"Storgatan 10","zip_city":"Falköping",
                 "plant_number":"ENC1","type":"Sophämtning","addressType":"next-pickup"}
            ],
            "Floby":[
                {"address":"Storgatan 1","zip_city":"Floby",
                 "plant_number":"ENC2","type":"Sophämtning","addressType":"next-pickup"}
            ]
        }"#;
        let hits = parse_search(body).unwrap();
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().any(|h| h.address == "Storgatan 10" && h.zip_city == "Falköping"));
        assert!(hits.iter().any(|h| h.address == "Storgatan 1" && h.zip_city == "Floby"));
    }

    #[test]
    fn parse_search_drops_hits_without_plant_number() {
        let body = r#"{"Skövde":[
            {"address":"","zip_city":"Skövde","plant_number":"X"},
            {"address":"Storgatan 1","zip_city":"Skövde","plant_number":""}
        ]}"#;
        assert!(parse_search(body).unwrap().is_empty());
    }

    #[test]
    fn parse_search_handles_empty_array_response() {
        let body = "[]";
        assert!(parse_search(body).unwrap().is_empty());
    }

    #[test]
    fn parse_pickup_groups_and_dedups_bins() {
        // Götene example — two identical Brännbart entries for same date.
        let body = r#"{
            "address":"Skolgatan 1","city":"Götene","plant_id":"X",
            "bins":[
                {"type":"Brännbart","pickup_date":"2026-09-21","formatted":"21 september"},
                {"type":"Brännbart","pickup_date":"2026-09-21","formatted":"21 september"},
                {"type":"Matavfall","pickup_date":"2026-09-14","formatted":"14 september"}
            ]
        }"#;
        let (addr, series) = parse_pickup(body).unwrap();
        assert_eq!(addr, "Skolgatan 1, Götene");
        assert_eq!(series.len(), 2);
        let bra = series.iter().find(|s| s.waste_type == "Brännbart").unwrap();
        assert_eq!(bra.anchor.len(), 1);
        assert!(bra.interval_weeks.is_none());
        let mat = series.iter().find(|s| s.waste_type == "Matavfall").unwrap();
        assert_eq!(
            mat.anchor[0],
            NaiveDate::from_ymd_opt(2026, 9, 14).unwrap()
        );
    }

    #[test]
    fn parse_pickup_handles_multi_fraction_response() {
        // Tibro example — 7 fractions on one address.
        let body = r#"{"address":"Skolgatan 10","city":"Tibro","bins":[
            {"type":"Brännbart","pickup_date":"2026-09-15","formatted":""},
            {"type":"Plast","pickup_date":"2026-09-17","formatted":""},
            {"type":"Kartong","pickup_date":"2026-09-17","formatted":""},
            {"type":"Metall","pickup_date":"2026-09-17","formatted":""},
            {"type":"Färgat glas","pickup_date":"2026-09-29","formatted":""},
            {"type":"Ofärgat glas","pickup_date":"2026-09-29","formatted":""},
            {"type":"Matavfall","pickup_date":"2026-09-15","formatted":""}
        ]}"#;
        let (_, series) = parse_pickup(body).unwrap();
        assert_eq!(series.len(), 7);
    }

    #[test]
    fn parse_pickup_ignores_invalid_dates_and_empty_types() {
        let body = r#"{"address":"X","city":"Y","bins":[
            {"type":"","pickup_date":"2026-09-15"},
            {"type":"Brännbart","pickup_date":""},
            {"type":"Brännbart","pickup_date":"not-a-date"},
            {"type":"Matavfall","pickup_date":"2026-09-15"}
        ]}"#;
        let (_, series) = parse_pickup(body).unwrap();
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].waste_type, "Matavfall");
    }

    #[test]
    fn city_filter_case_insensitive_unicode() {
        let p = Avfallsappen::new(reqwest::Client::new(), cfg(&["Skövde", "Timmersdala"]));
        assert!(p.matches_city("skövde"));
        assert!(p.matches_city("SKÖVDE"));
        assert!(p.matches_city("Timmersdala"));
        assert!(!p.matches_city("Falköping"));
    }

    #[test]
    fn split_input_full_form() {
        let (s, c) = split_input("Storgatan 10, Falköping");
        assert_eq!(s, "Storgatan 10");
        assert_eq!(c.as_deref(), Some("Falköping"));
    }

    #[test]
    fn split_input_street_only() {
        let (s, c) = split_input("Storgatan 10");
        assert_eq!(s, "Storgatan 10");
        assert!(c.is_none());
    }

    #[test]
    fn split_input_trailing_comma_no_city() {
        let (s, c) = split_input("Storgatan 10,");
        assert_eq!(s, "Storgatan 10");
        assert!(c.is_none());
    }
}
