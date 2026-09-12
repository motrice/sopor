//! Alvesta Renhållnings AB (ARAB) — publikt schema-API på
//! `arabschema.alvesta.se`. SPA:n (Vite/React) laddar ner hela datasetet
//! i förväg via `GET /app/data/{isAuthorized}` (~2,5 MB, ~6700 adresser)
//! och filtrerar lokalt. Vi speglar den strategin: cache i minnet med
//! 12 h TTL och substring-autocomplete.
//!
//! Endpoint-parametern är `isAuthorized` (bool) från SPA:ns route-state.
//! Servern ignorerar värdet — både `/false` och `/true` returnerar samma
//! JSON. Vi använder `/false` för att inte se ut som en inloggad request.
//!
//! Struktur per post:
//!   { address: "Sissleboda 4, Sissleboda 1:8",
//!     dateFormat: "1-21-",
//!     schemes: [ { week: 39,
//!                  date: "måndag, september 21, 2026",
//!                  dayOff: false } ] }
//!
//! `address` är en fritextsträng där gatuadress och fastighetsbeteckning
//! separeras av komma. `dateFormat` = `<dow>-<startvecka>-[ja]` (dow 1=mån
//! …5=fre). Datumen står i schemes[].date i svensk longform och det är
//! den vi parsar. Endast en avfallsström per adress (hushållskärl).
//! Upstream publicerar bara ~5 kommande datum → vi emitterar explicita
//! datum utan RRULE och förlitar oss på klientens refresh (samma mönster
//! som Hässleholm och SRV).

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Deserialize;
use tokio::sync::RwLock;
use tokio::time::Instant;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const DATASET_URL: &str = "https://arabschema.alvesta.se/app/data/false";
const TTL: Duration = Duration::from_secs(12 * 3600);
const AUTOCOMPLETE_LIMIT: usize = 25;

pub struct Alvesta {
    http: reqwest::Client,
    cache: RwLock<Option<Arc<Dataset>>>,
}

struct Dataset {
    fetched_at: Instant,
    entries: Vec<RawEntry>,
}

#[derive(Deserialize, Debug, Clone)]
struct RawResponse {
    #[serde(default)]
    schedule: Vec<RawEntry>,
}

#[derive(Deserialize, Debug, Clone)]
struct RawEntry {
    #[serde(default)]
    address: String,
    #[serde(default)]
    schemes: Vec<RawScheme>,
}

#[derive(Deserialize, Debug, Clone)]
struct RawScheme {
    #[serde(default)]
    date: String,
}

impl Alvesta {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            cache: RwLock::new(None),
        }
    }

    async fn dataset(&self) -> Result<Arc<Dataset>, ProviderError> {
        if let Some(d) = self.cache.read().await.as_ref() {
            if d.fetched_at.elapsed() < TTL {
                return Ok(d.clone());
            }
        }
        let mut guard = self.cache.write().await;
        if let Some(d) = guard.as_ref() {
            if d.fetched_at.elapsed() < TTL {
                return Ok(d.clone());
            }
        }
        let resp: RawResponse = self
            .http
            .get(DATASET_URL)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let arc = Arc::new(Dataset {
            fetched_at: Instant::now(),
            entries: resp.schedule,
        });
        *guard = Some(arc.clone());
        Ok(arc)
    }
}

fn swedish_month(name: &str) -> Option<u32> {
    match name.to_lowercase().as_str() {
        "januari" => Some(1),
        "februari" => Some(2),
        "mars" => Some(3),
        "april" => Some(4),
        "maj" => Some(5),
        "juni" => Some(6),
        "juli" => Some(7),
        "augusti" => Some(8),
        "september" => Some(9),
        "oktober" => Some(10),
        "november" => Some(11),
        "december" => Some(12),
        _ => None,
    }
}

// "måndag, september 21, 2026" → 2026-09-21.
fn parse_swedish_date(s: &str) -> Option<NaiveDate> {
    let (_weekday, rest) = s.split_once(',')?;
    let (month_day, year) = rest.trim().rsplit_once(',')?;
    let year: i32 = year.trim().parse().ok()?;
    let (month, day) = month_day.trim().split_once(' ')?;
    let month = swedish_month(month.trim())?;
    let day: u32 = day.trim().parse().ok()?;
    NaiveDate::from_ymd_opt(year, month, day)
}

fn cadence_text(dates: &[NaiveDate]) -> String {
    if dates.len() < 2 {
        return String::new();
    }
    let mut diffs = dates.windows(2).map(|w| (w[1] - w[0]).num_days());
    let first = diffs.next().unwrap();
    if !diffs.all(|d| d == first) {
        return String::new();
    }
    match first {
        7 => "Varje vecka".into(),
        14 => "Varannan vecka".into(),
        28 => "Var fjärde vecka".into(),
        _ => String::new(),
    }
}

fn build_series(entry: &RawEntry) -> Vec<PickupSeries> {
    let mut dates: Vec<NaiveDate> = entry
        .schemes
        .iter()
        .filter_map(|s| parse_swedish_date(&s.date))
        .collect();
    dates.sort();
    dates.dedup();
    if dates.is_empty() {
        return Vec::new();
    }
    vec![PickupSeries {
        waste_type: "Hushållsavfall".into(),
        frequency_text: cadence_text(&dates),
        interval_weeks: None,
        anchor: dates,
    }]
}

#[async_trait]
impl Provider for Alvesta {
    fn id(&self) -> &'static str {
        "alvesta"
    }
    fn name(&self) -> &'static str {
        "Alvesta"
    }
    fn placeholder(&self) -> &'static str {
        "t.ex. Ågårdsvägen 31 eller fastighetsbeteckning"
    }
    fn note(&self) -> &'static str {
        "Sophämtningsdata från Alvesta Renhållnings AB (ARAB). Sök på \
         gatuadress eller fastighetsbeteckning — datasetet innehåller \
         båda. ARAB publicerar bara närmaste tömningarna per adress, så \
         kalendern fylls på löpande när klienten refreshar prenumerationen."
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
            if entry.address.is_empty() {
                continue;
            }
            if !entry.address.to_lowercase().contains(&q) {
                continue;
            }
            if seen.insert(entry.address.clone()) {
                out.push(Suggestion {
                    value: entry.address.clone(),
                });
                if out.len() >= AUTOCOMPLETE_LIMIT {
                    break;
                }
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let dataset = self.dataset().await?;
        let needle = address.trim().to_lowercase();
        let entry = dataset
            .entries
            .iter()
            .find(|e| e.address.to_lowercase() == needle);
        let series = entry.map(build_series).unwrap_or_default();
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
    fn parse_date_basic() {
        assert_eq!(
            parse_swedish_date("måndag, september 21, 2026"),
            Some(NaiveDate::from_ymd_opt(2026, 9, 21).unwrap())
        );
        assert_eq!(
            parse_swedish_date("fredag, december 04, 2026"),
            Some(NaiveDate::from_ymd_opt(2026, 12, 4).unwrap())
        );
    }

    #[test]
    fn parse_date_rejects_garbage() {
        assert!(parse_swedish_date("").is_none());
        assert!(parse_swedish_date("måndag").is_none());
        assert!(parse_swedish_date("måndag, foobar 21, 2026").is_none());
        assert!(parse_swedish_date("måndag, september 99, 2026").is_none());
    }

    #[test]
    fn cadence_biweekly() {
        let dates = vec![
            NaiveDate::from_ymd_opt(2026, 9, 21).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 5).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 19).unwrap(),
        ];
        assert_eq!(cadence_text(&dates), "Varannan vecka");
    }

    #[test]
    fn cadence_four_weekly() {
        let dates = vec![
            NaiveDate::from_ymd_opt(2026, 9, 21).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 19).unwrap(),
        ];
        assert_eq!(cadence_text(&dates), "Var fjärde vecka");
    }

    #[test]
    fn cadence_irregular_is_empty() {
        let dates = vec![
            NaiveDate::from_ymd_opt(2026, 9, 21).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 5).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 26).unwrap(),
        ];
        assert_eq!(cadence_text(&dates), "");
    }

    #[test]
    fn build_series_from_sample() {
        let entry = RawEntry {
            address: "Sissleboda 4, Sissleboda 1:8".into(),
            schemes: vec![
                RawScheme {
                    date: "måndag, september 21, 2026".into(),
                },
                RawScheme {
                    date: "måndag, oktober 05, 2026".into(),
                },
                RawScheme {
                    date: "måndag, oktober 19, 2026".into(),
                },
            ],
        };
        let series = build_series(&entry);
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].waste_type, "Hushållsavfall");
        assert_eq!(series[0].frequency_text, "Varannan vecka");
        assert!(series[0].interval_weeks.is_none());
        assert_eq!(series[0].anchor.len(), 3);
        assert_eq!(series[0].anchor[0], NaiveDate::from_ymd_opt(2026, 9, 21).unwrap());
    }

    #[test]
    fn build_series_skips_invalid_dates() {
        let entry = RawEntry {
            address: "X".into(),
            schemes: vec![
                RawScheme {
                    date: "not-a-date".into(),
                },
                RawScheme {
                    date: "måndag, oktober 05, 2026".into(),
                },
            ],
        };
        let series = build_series(&entry);
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].anchor.len(), 1);
    }

    #[test]
    fn build_series_empty_when_no_valid_dates() {
        let entry = RawEntry {
            address: "X".into(),
            schemes: vec![RawScheme {
                date: "junk".into(),
            }],
        };
        assert!(build_series(&entry).is_empty());
    }

    #[test]
    fn build_series_dedups() {
        let entry = RawEntry {
            address: "X".into(),
            schemes: vec![
                RawScheme {
                    date: "måndag, oktober 05, 2026".into(),
                },
                RawScheme {
                    date: "måndag, oktober 05, 2026".into(),
                },
            ],
        };
        let series = build_series(&entry);
        assert_eq!(series[0].anchor.len(), 1);
    }

    #[test]
    fn deserializes_dataset_sample() {
        let json = r#"{
            "schedule": [
                {
                    "address": "Sissleboda Lindhem, Sissleboda 1:4",
                    "dateFormat": "1-21-",
                    "schemes": [
                        {"week": 39, "date": "måndag, september 21, 2026", "dayOff": false},
                        {"week": 41, "date": "måndag, oktober 05, 2026", "dayOff": false}
                    ]
                }
            ]
        }"#;
        let resp: RawResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.schedule.len(), 1);
        assert_eq!(resp.schedule[0].address, "Sissleboda Lindhem, Sissleboda 1:4");
        assert_eq!(resp.schedule[0].schemes.len(), 2);
    }
}
