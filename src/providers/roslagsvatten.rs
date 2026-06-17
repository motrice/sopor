//! Roslagsvatten — Drupal-based widget covering Ekerö, Vaxholm and
//! Österåker kommuner (replaced their previous FutureWeb deployment).
//!
//! Two open POST JSON endpoints, no auth:
//!   POST /schedule/search  { searchText, municipality }
//!     -> Drupal AJAX array: [{ data: "<ul>...<li data-bid='...'>Addr</li>...</ul>", ... }]
//!   POST /schedule/fetch   { buildingId, municipality }
//!     -> Drupal AJAX array with embedded HTML fragments:
//!        <h3>Restavfall</h3>
//!        <p>Frekvens: Torsdag udda vecka. Avfall</p>
//!        <p>Nästa hämtning: 2026-06-18</p>

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::NaiveDate;
use regex::Regex;
use serde::Deserialize;
use serde_json::json;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const SEARCH_URL: &str = "https://roslagsvatten.se/schedule/search";
const FETCH_URL: &str = "https://roslagsvatten.se/schedule/fetch";

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    /// Upstream radio-button value: "ekero", "vaxholm", or "osteraker".
    pub municipality: &'static str,
}

pub struct Roslagsvatten {
    http: reqwest::Client,
    cfg: Config,
}

impl Roslagsvatten {
    pub fn new(http: reqwest::Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    async fn search(&self, query: &str) -> Result<Vec<AddressHit>, ProviderError> {
        let body = json!({"searchText": query, "municipality": self.cfg.municipality});
        let resp = self
            .http
            .post(SEARCH_URL)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await?;
        // The search endpoint returns HTML 500 when no addresses match
        // even though the response body might still be parseable. Bail
        // out as empty on any non-2xx.
        if !resp.status().is_success() {
            return Ok(vec![]);
        }
        let drupal: Vec<DrupalCommand> = resp.json().await?;
        let mut hits = Vec::new();
        for cmd in drupal {
            if let Some(data) = cmd.data {
                hits.extend(parse_address_list(&data));
            }
        }
        Ok(hits)
    }

    async fn fetch(&self, building_id: &str) -> Result<Vec<PickupSeries>, ProviderError> {
        let body = json!({"buildingId": building_id, "municipality": self.cfg.municipality});
        let resp = self
            .http
            .post(FETCH_URL)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Ok(vec![]);
        }
        let drupal: Vec<DrupalCommand> = resp.json().await?;
        let mut series = Vec::new();
        for cmd in drupal {
            if let Some(data) = cmd.data {
                series.extend(parse_schedule_fragment(&data));
            }
        }
        Ok(series)
    }
}

#[derive(Deserialize)]
struct DrupalCommand {
    #[serde(default)]
    data: Option<String>,
}

#[derive(Debug, Clone)]
struct AddressHit {
    address: String,
    building_id: String,
}

fn parse_address_list(html: &str) -> Vec<AddressHit> {
    // <li ... data-bid="ID" class="waste-schedule-address-item">Address</li>
    let re = Regex::new(
        r#"(?s)<li[^>]*data-bid=["']([^"']+)["'][^>]*class="[^"]*waste-schedule-address-item[^"]*"[^>]*>([^<]+)</li>"#,
    )
    .unwrap();
    re.captures_iter(html)
        .map(|c| AddressHit {
            building_id: c.get(1).unwrap().as_str().to_string(),
            address: c.get(2).unwrap().as_str().trim().to_string(),
        })
        .collect()
}

fn parse_schedule_fragment(html: &str) -> Vec<PickupSeries> {
    // The fragment contains repeated:
    //   <div class="waste-schedule-inner">
    //     <h3>Restavfall</h3>
    //     <p>Frekvens: Torsdag udda vecka. Avfall</p>
    //     <p>Nästa hämtning: 2026-06-18</p>
    //   </div>
    let block_re = Regex::new(
        r#"(?s)<div[^>]*class="[^"]*waste-schedule-inner[^"]*"[^>]*>(.*?)</div>"#,
    )
    .unwrap();
    let h3_re = Regex::new(r#"(?s)<h3[^>]*>([^<]+)</h3>"#).unwrap();
    let freq_re = Regex::new(r#"(?s)Frekvens:\s*([^<]+?)</p>"#).unwrap();
    let next_re = Regex::new(r#"(?s)Nästa hämtning:\s*(\d{4}-\d{2}-\d{2})"#).unwrap();

    let mut by_type: BTreeMap<String, (String, Vec<NaiveDate>)> = BTreeMap::new();
    for block in block_re.captures_iter(html) {
        let inner = block.get(1).unwrap().as_str();
        let waste_type = h3_re
            .captures(inner)
            .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
            .unwrap_or_default();
        if waste_type.is_empty() {
            continue;
        }
        let frequency = freq_re
            .captures(inner)
            .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
            .unwrap_or_default();
        let date = next_re
            .captures(inner)
            .and_then(|c| c.get(1).map(|m| m.as_str()))
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
        let Some(date) = date else { continue };
        let entry = by_type
            .entry(waste_type)
            .or_insert_with(|| (frequency.clone(), Vec::new()));
        if entry.0.is_empty() {
            entry.0 = frequency;
        }
        entry.1.push(date);
    }

    let mut series = Vec::new();
    for (waste_type, (frequency_text, mut dates)) in by_type {
        dates.sort();
        dates.dedup();
        let interval_weeks = parse_interval(&frequency_text);
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

fn parse_interval(freq: &str) -> Option<u32> {
    let lower = freq.to_lowercase();
    // "Torsdag udda vecka" / "Torsdag jämn vecka" → every other week.
    if lower.contains("udda vecka") || lower.contains("jämn vecka") {
        return Some(2);
    }
    if lower.contains("varje vecka") {
        return Some(1);
    }
    if lower.contains("varannan vecka") {
        return Some(2);
    }
    // "Var fjärde vecka" / "var 4:e vecka"
    if lower.contains("fjärde vecka") {
        return Some(4);
    }
    let digits: String = lower
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if !digits.is_empty() && lower.contains("vecka") {
        return digits.parse().ok();
    }
    None
}

#[async_trait]
impl Provider for Roslagsvatten {
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
        if q.len() < 3 {
            // Drupal's debounce enforces 3+ chars; the search endpoint
            // returns HTTP 500 for shorter inputs.
            return Ok(vec![]);
        }
        let hits = self.search(q).await?;
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for h in hits {
            if seen.insert(h.address.clone()) {
                out.push(Suggestion { value: h.address });
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        // Re-run the search to find the building id for this address. Use a
        // generous prefix so the upstream returns the entry.
        let prefix: String = address.chars().take(20).collect();
        let q = prefix.trim();
        if q.len() < 3 {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        }
        let hits = self.search(q).await?;
        let Some(hit) = hits.into_iter().find(|h| h.address.eq_ignore_ascii_case(address)) else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };
        let series = self.fetch(&hit.building_id).await?;
        Ok(PickupSchedule {
            address: address.to_string(),
            series,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_address_list() {
        let html = r#"<ul id="waste-schedule-building-list">
            <li id="waste-schedule-address-item-1" data-bid="111006001" class="waste-schedule-address-item">Andromedavägen 1, Åkersberga</li>
            <li id="waste-schedule-address-item-2" data-bid="111006502" class="waste-schedule-address-item">Andromedavägen 2, Åkersberga</li>
        </ul>"#;
        let hits = parse_address_list(html);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].building_id, "111006001");
        assert_eq!(hits[0].address, "Andromedavägen 1, Åkersberga");
    }

    #[test]
    fn parses_schedule_fragment_with_two_types() {
        let html = r#"<div class="waste-schedule-results">
            <div class="waste-schedule-inner">
                <h3>Restavfall</h3>
                <p>Frekvens: Torsdag udda vecka. Avfall</p>
                <p>Nästa hämtning: 2026-06-18</p>
            </div>
            <div class="waste-schedule-inner">
                <h3>Matavfall</h3>
                <p>Frekvens: Torsdag varje vecka</p>
                <p>Nästa hämtning: 2026-06-18</p>
            </div>
        </div>"#;
        let series = parse_schedule_fragment(html);
        assert_eq!(series.len(), 2);
        let mat = series.iter().find(|s| s.waste_type == "Matavfall").unwrap();
        assert_eq!(mat.interval_weeks, Some(1));
        let rest = series.iter().find(|s| s.waste_type == "Restavfall").unwrap();
        assert_eq!(rest.interval_weeks, Some(2));
        assert_eq!(rest.anchor.len(), 1);
        assert_eq!(
            rest.anchor[0],
            NaiveDate::from_ymd_opt(2026, 6, 18).unwrap()
        );
    }

    #[test]
    fn parse_interval_swedish_forms() {
        assert_eq!(parse_interval("Torsdag udda vecka. Avfall"), Some(2));
        assert_eq!(parse_interval("Torsdag jämn vecka"), Some(2));
        assert_eq!(parse_interval("Onsdag varje vecka"), Some(1));
        assert_eq!(parse_interval("varannan vecka"), Some(2));
        assert_eq!(parse_interval("var fjärde vecka"), Some(4));
        assert_eq!(parse_interval("var 8:e vecka"), Some(8));
        assert_eq!(parse_interval("ad hoc"), None);
    }

    #[test]
    fn schedule_fragment_skips_blocks_without_date() {
        let html = r#"<div class="waste-schedule-inner">
            <h3>Slam</h3>
            <p>Frekvens: Vid behov</p>
            <p>Ingen planerad hämtning.</p>
        </div>"#;
        assert!(parse_schedule_fragment(html).is_empty());
    }
}
