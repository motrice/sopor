//! Sundsvall — open-data garbage schedule (CC0 1.0, dataset published on
//! dataportal.se: `61_6752`). Documented OpenAPI at
//! `https://api.sundsvall.se/Garbage/api-docs`.
//!
//! The endpoint `GET /Garbage/{municipalityId}/schedules` returns every
//! address's schedule when called without filter parameters. For Sundsvall
//! (`municipalityId = 2281`) the dataset is ~12 MB / ~23k records / 1.4k
//! unique streets. We fetch the whole thing into memory once per 12h and
//! filter locally — querying the upstream with just a street name returns
//! empty results, so a postalCode-less per-request lookup is not viable.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Deserialize;
use tokio::sync::RwLock;
use tokio::time::Instant;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const DATASET_URL: &str = "https://api.sundsvall.se/Garbage/2281/schedules";
const TTL: Duration = Duration::from_secs(12 * 3600);
const AUTOCOMPLETE_LIMIT: usize = 25;

pub struct Sundsvall {
    http: reqwest::Client,
    cache: RwLock<Option<Arc<Dataset>>>,
}

struct Dataset {
    fetched_at: Instant,
    entries: Vec<RawEntry>,
}

#[derive(Deserialize, Debug, Clone)]
struct RawEntry {
    address: RawAddress,
    #[serde(rename = "additionalInformation", default)]
    additional_information: String,
    #[serde(default)]
    schedules: Vec<RawSchedule>,
}

#[derive(Deserialize, Debug, Clone)]
struct RawAddress {
    #[serde(default)]
    street: String,
    #[serde(rename = "houseNumber", default)]
    house_number: String,
    #[serde(rename = "postalCode", default)]
    postal_code: String,
    #[serde(default)]
    city: String,
}

#[derive(Deserialize, Debug, Clone)]
struct RawSchedule {
    #[serde(rename = "nextPickupDate", default)]
    next_pickup_date: String,
    #[serde(rename = "wasteType", default)]
    waste_type: String,
}

impl Sundsvall {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            cache: RwLock::new(None),
        }
    }

    async fn dataset(&self) -> Result<Arc<Dataset>, ProviderError> {
        // Fast path: read lock, hit cache if fresh.
        if let Some(d) = self.cache.read().await.as_ref() {
            if d.fetched_at.elapsed() < TTL {
                return Ok(d.clone());
            }
        }
        // Slow path: write lock + double-check + refresh. Other requests block
        // for ~1–2 s during the periodic refetch.
        let mut guard = self.cache.write().await;
        if let Some(d) = guard.as_ref() {
            if d.fetched_at.elapsed() < TTL {
                return Ok(d.clone());
            }
        }
        let entries: Vec<RawEntry> = self
            .http
            .get(DATASET_URL)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let arc = Arc::new(Dataset {
            fetched_at: Instant::now(),
            entries,
        });
        *guard = Some(arc.clone());
        Ok(arc)
    }
}

fn address_label(a: &RawAddress, additional: &str) -> String {
    let info = if additional.trim().is_empty() {
        String::new()
    } else {
        format!(" {}", additional.trim())
    };
    if a.postal_code.is_empty() {
        format!("{} {}{}, {}", a.street, a.house_number, info, a.city)
    } else {
        format!(
            "{} {}{}, {} {}",
            a.street, a.house_number, info, a.city, a.postal_code
        )
    }
}

fn waste_type_label(code: &str) -> String {
    match code {
        "WASTE" => "Restavfall".into(),
        "FOOD" => "Matavfall".into(),
        "PAPER" => "Pappersförpackningar".into(),
        "PLASTIC" => "Plastförpackningar".into(),
        other => other.to_string(),
    }
}

fn build_series(entry: &RawEntry) -> Vec<PickupSeries> {
    // (waste_type_label) -> set of dates (earliest first)
    let mut by_type: BTreeMap<String, HashSet<NaiveDate>> = BTreeMap::new();
    for s in &entry.schedules {
        let Ok(date) = NaiveDate::parse_from_str(&s.next_pickup_date, "%Y-%m-%d") else {
            continue;
        };
        by_type
            .entry(waste_type_label(&s.waste_type))
            .or_default()
            .insert(date);
    }

    let mut series = Vec::new();
    for (waste_type, dates) in by_type {
        let mut dates: Vec<NaiveDate> = dates.into_iter().collect();
        dates.sort();
        if dates.is_empty() {
            continue;
        }
        // Each address-level garbageScheduledWeek of ODD/EVEN signals
        // biweekly cadence in principle, but the upstream only returns the
        // single "next" date per waste type and the actual cadence differs
        // per type. Emit explicit dates without RRULE; the calendar client
        // refetches and picks up the next nextPickupDate after each pickup.
        series.push(PickupSeries {
            waste_type,
            frequency_text: String::new(),
            interval_weeks: None,
            anchor: dates,
        });
    }
    series
}

#[async_trait]
impl Provider for Sundsvall {
    fn id(&self) -> &'static str {
        "sundsvall"
    }
    fn name(&self) -> &'static str {
        "Sundsvall"
    }
    fn placeholder(&self) -> &'static str {
        "t.ex. Storgatan 1"
    }
    fn note(&self) -> &'static str {
        "Officiell öppen data (CC0) från Sundsvalls kommun via \
         dataportal.se. Endast Sundsvalls kommun — Timrå och \
         Nordanstig saknas i datasetet."
    }

    async fn autocomplete(&self, query: &str) -> Result<Vec<Suggestion>, ProviderError> {
        let q = query.trim().to_lowercase();
        if q.len() < 2 {
            return Ok(vec![]);
        }
        let dataset = self.dataset().await?;
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for entry in &dataset.entries {
            let label = address_label(&entry.address, &entry.additional_information);
            let lower = label.to_lowercase();
            if !lower.contains(&q) {
                continue;
            }
            if seen.insert(label.clone()) {
                out.push(Suggestion { value: label });
                if out.len() >= AUTOCOMPLETE_LIMIT {
                    break;
                }
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let dataset = self.dataset().await?;
        let needle = address.to_lowercase();
        let Some(entry) = dataset.entries.iter().find(|e| {
            address_label(&e.address, &e.additional_information).to_lowercase() == needle
        }) else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };
        Ok(PickupSchedule {
            address: address.to_string(),
            series: build_series(entry),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> RawEntry {
        RawEntry {
            address: RawAddress {
                street: "Trossvägen".into(),
                house_number: "3".into(),
                postal_code: "86533".into(),
                city: "Alnö".into(),
            },
            additional_information: String::new(),
            schedules: vec![
                RawSchedule {
                    next_pickup_date: "2026-07-21".into(),
                    waste_type: "WASTE".into(),
                },
                RawSchedule {
                    next_pickup_date: "2026-07-21".into(),
                    waste_type: "WASTE".into(),
                },
                RawSchedule {
                    next_pickup_date: "2026-06-23".into(),
                    waste_type: "FOOD".into(),
                },
                RawSchedule {
                    next_pickup_date: "2026-06-23".into(),
                    waste_type: "PAPER".into(),
                },
                RawSchedule {
                    next_pickup_date: "2026-07-07".into(),
                    waste_type: "PLASTIC".into(),
                },
            ],
        }
    }

    #[test]
    fn label_basic() {
        assert_eq!(
            address_label(&sample_entry().address, ""),
            "Trossvägen 3, Alnö 86533"
        );
    }

    #[test]
    fn label_with_additional_information() {
        assert_eq!(
            address_label(&sample_entry().address, "A"),
            "Trossvägen 3 A, Alnö 86533"
        );
    }

    #[test]
    fn label_without_postcode() {
        let mut a = sample_entry().address;
        a.postal_code = String::new();
        assert_eq!(address_label(&a, ""), "Trossvägen 3, Alnö");
    }

    #[test]
    fn waste_type_labels() {
        assert_eq!(waste_type_label("WASTE"), "Restavfall");
        assert_eq!(waste_type_label("FOOD"), "Matavfall");
        assert_eq!(waste_type_label("PAPER"), "Pappersförpackningar");
        assert_eq!(waste_type_label("PLASTIC"), "Plastförpackningar");
        assert_eq!(waste_type_label("UNKNOWN"), "UNKNOWN");
    }

    #[test]
    fn build_series_dedupes_and_orders() {
        let series = build_series(&sample_entry());
        // 4 distinct waste types after dedup (WASTE duplicate same date)
        assert_eq!(series.len(), 4);
        let rest = series.iter().find(|s| s.waste_type == "Restavfall").unwrap();
        assert_eq!(
            rest.anchor,
            vec![NaiveDate::from_ymd_opt(2026, 7, 21).unwrap()]
        );
        assert!(rest.interval_weeks.is_none());
        let plastic = series
            .iter()
            .find(|s| s.waste_type == "Plastförpackningar")
            .unwrap();
        assert_eq!(
            plastic.anchor,
            vec![NaiveDate::from_ymd_opt(2026, 7, 7).unwrap()]
        );
    }

    #[test]
    fn build_series_skips_invalid_dates() {
        let mut e = sample_entry();
        e.schedules = vec![RawSchedule {
            next_pickup_date: "not-a-date".into(),
            waste_type: "WASTE".into(),
        }];
        assert!(build_series(&e).is_empty());
    }

    #[test]
    fn raw_entry_deserializes_from_dataset_sample() {
        let json = r#"{
            "additionalInformation": "",
            "address": {
                "city": "Alnö",
                "houseNumber": "3",
                "postalCode": "86533",
                "street": "Trossvägen"
            },
            "facilityCategory": "VILLA",
            "garbageScheduledDay": "TUESDAY",
            "garbageScheduledWeek": "EVEN",
            "schedules": [
                {"nextPickupDate":"2026-07-21","wasteType":"WASTE"},
                {"nextPickupDate":"2026-06-23","wasteType":"FOOD"}
            ]
        }"#;
        let e: RawEntry = serde_json::from_str(json).unwrap();
        assert_eq!(e.address.street, "Trossvägen");
        assert_eq!(e.schedules.len(), 2);
    }
}
