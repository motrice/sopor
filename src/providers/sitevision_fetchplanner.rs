//! Generic provider for kommuner using Limepark's SiteVision FetchPlanner
//! widget. Same data shape across deployments (`limepark.app-fetchplanner` and
//! the older `limepark.webapp-fetchplanner`), only the URL, portlet id and
//! kommun label differ.
//!
//! Confirmed deployments: Falu Energi & Vatten (Falun), Miva (Örnsköldsvik).

use std::collections::{BTreeMap, HashSet};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Europe::Stockholm as StockholmTz;
use serde::Deserialize;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    pub portlet_id: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    pub default_city: &'static str,
}

pub struct SitevisionFetchplanner {
    http: reqwest::Client,
    cfg: Config,
}

impl SitevisionFetchplanner {
    pub fn new(http: reqwest::Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    async fn fetch_state(&self, query: &str) -> Result<InitialState, ProviderError> {
        let body = self
            .http
            .get(self.cfg.url)
            .query(&[("q", query)])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        extract_state(&body, self.cfg.portlet_id)
    }
}

#[derive(Deserialize, Debug, Default)]
struct InitialState {
    #[serde(default)]
    hits: Vec<Hit>,
    #[serde(default)]
    containers: Vec<Container>,
    #[serde(default)]
    trips: Vec<Trip>,
    #[serde(default)]
    address: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Hit {
    pickup_address: String,
    #[serde(default)]
    pickup_city: String,
    #[serde(default)]
    pickup_zip_code: String,
    content_type_name: String,
    calendars: Vec<HitCalendar>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct HitCalendar {
    execution_date: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Container {
    type_text: String,
    #[serde(default)]
    pickup_date_iso: String,
    #[serde(default)]
    next_pickup_date_iso: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Trip {
    #[serde(default)]
    containers: Vec<TripContainer>,
    #[serde(default)]
    pickup_date_iso: String,
    #[serde(default)]
    next_pickup_date_iso: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TripContainer {
    type_text: String,
}

fn extract_state(html: &str, portlet_id: &str) -> Result<InitialState, ProviderError> {
    let needle = format!("registerInitialState('{portlet_id}',");
    let start = html
        .find(&needle)
        .ok_or_else(|| ProviderError("missing fetchplanner state".into()))?
        + needle.len();
    let json_part = &html[start..];
    let mut de = serde_json::Deserializer::from_str(json_part);
    InitialState::deserialize(&mut de).map_err(|e| ProviderError(format!("parse: {e}")))
}

fn iso_to_local_date(iso: &str) -> Option<NaiveDate> {
    if iso.is_empty() {
        return None;
    }
    let dt: DateTime<Utc> = DateTime::parse_from_rfc3339(iso).ok()?.with_timezone(&Utc);
    Some(dt.with_timezone(&StockholmTz).date_naive())
}

fn titlecase_city(city: &str) -> String {
    let mut out = String::with_capacity(city.len());
    let mut first = true;
    for c in city.chars() {
        if first {
            out.extend(c.to_uppercase());
            first = false;
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

fn format_label(addr: &str, city: &str, zip: &str, default_city: &str) -> String {
    let city = if city.is_empty() {
        default_city.to_string()
    } else {
        titlecase_city(city)
    };
    if zip.is_empty() {
        format!("{addr}, {city}")
    } else {
        format!("{addr}, {city} {zip}")
    }
}

#[async_trait]
impl Provider for SitevisionFetchplanner {
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
        let state = self.fetch_state(q).await?;

        let mut suggestions = Vec::new();
        let mut seen = HashSet::new();

        for h in &state.hits {
            let value = format_label(
                &h.pickup_address,
                &h.pickup_city,
                &h.pickup_zip_code,
                self.cfg.default_city,
            );
            if seen.insert(value.clone()) {
                suggestions.push(Suggestion { value });
            }
        }

        if suggestions.is_empty() {
            if let Some(addr) = state.address {
                if !state.containers.is_empty() {
                    let value = format!("{addr}, {}", self.cfg.default_city);
                    suggestions.push(Suggestion { value });
                }
            }
        }

        Ok(suggestions)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let street_part = address
            .split(',')
            .next()
            .unwrap_or(address)
            .trim()
            .to_string();
        let state = self.fetch_state(&street_part).await?;

        let mut series_map: BTreeMap<String, Vec<NaiveDate>> = BTreeMap::new();

        if !state.hits.is_empty() {
            for h in state.hits {
                if !addresses_match(
                    address,
                    &h.pickup_address,
                    &h.pickup_city,
                    &h.pickup_zip_code,
                    self.cfg.default_city,
                ) {
                    continue;
                }
                let entry = series_map.entry(h.content_type_name.clone()).or_default();
                for cal in h.calendars {
                    if let Some(d) = iso_to_local_date(&cal.execution_date) {
                        entry.push(d);
                    }
                }
            }
        } else {
            // Resolved-view path: some deployments put ISO dates on the
            // container itself (Falun), others put them on trips[] keyed by
            // container typeText (Miva). Walk both and dedup later.
            for c in state.containers {
                if c.pickup_date_iso.is_empty() && c.next_pickup_date_iso.is_empty() {
                    continue;
                }
                let entry = series_map.entry(c.type_text.clone()).or_default();
                if let Some(d) = iso_to_local_date(&c.pickup_date_iso) {
                    entry.push(d);
                }
                if let Some(d) = iso_to_local_date(&c.next_pickup_date_iso) {
                    entry.push(d);
                }
            }
            for trip in state.trips {
                let pickup = iso_to_local_date(&trip.pickup_date_iso);
                let next = iso_to_local_date(&trip.next_pickup_date_iso);
                if pickup.is_none() && next.is_none() {
                    continue;
                }
                let mut seen_types = HashSet::new();
                for tc in trip.containers {
                    if !seen_types.insert(tc.type_text.clone()) {
                        continue;
                    }
                    let entry = series_map.entry(tc.type_text).or_default();
                    if let Some(d) = pickup {
                        entry.push(d);
                    }
                    if let Some(d) = next {
                        entry.push(d);
                    }
                }
            }
        }

        let mut series = Vec::new();
        for (waste_type, mut dates) in series_map {
            dates.sort();
            dates.dedup();
            let interval = if dates.len() >= 2 {
                let gap = (dates[1] - dates[0]).num_days();
                if gap > 0 && gap % 7 == 0 {
                    Some((gap / 7) as u32)
                } else {
                    None
                }
            } else {
                None
            };
            let frequency_text = match interval {
                Some(1) => "Varje vecka".to_string(),
                Some(2) => "Varannan vecka".to_string(),
                Some(n) => format!("Var {n}:e vecka"),
                None => String::new(),
            };
            series.push(PickupSeries {
                waste_type,
                frequency_text,
                interval_weeks: None,
                anchor: dates,
            });
        }

        Ok(PickupSchedule {
            address: address.to_string(),
            series,
        })
    }
}

fn addresses_match(
    requested: &str,
    hit_addr: &str,
    hit_city: &str,
    hit_zip: &str,
    default_city: &str,
) -> bool {
    let normalized_hit = format_label(hit_addr, hit_city, hit_zip, default_city);
    if normalized_hit.eq_ignore_ascii_case(requested) {
        return true;
    }
    let req_street = requested.split(',').next().unwrap_or(requested).trim();
    req_street.eq_ignore_ascii_case(hit_addr)
}
