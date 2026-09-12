//! Vatten och miljöresurs (VMR) — Härjedalen, Berg, Bräcke.
//!
//! SiteVision-webbapp `garbage-collection` på vattenmiljoresurs.se
//! serverar hela adressdatasetet anonymt via portlet-route
//! `/allAddresses`. Per kommun finns en egen portlet-UUID; datasetet
//! är disjunkt.
//!
//! Endpoint:
//!   GET https://www.vattenmiljoresurs.se/<path>?sv.target=<uuid>
//!       &sv.<uuid>.route=/allAddresses&svAjaxReqParam=ajax
//!
//! Kräver header `X-Requested-With: XMLHttpRequest` — annars returnerar
//! SiteVision hela HTML-sidan istället för JSON-svar.
//!
//! Datasetet innehåller `{ ort, hamtstalle, tbfurenhkorlistarad_strtomningsfrekvenskod }`.
//! Frekvenskoden `h<intervall><parity><veckodag>` avkodas lokalt av
//! widgeten och används för att generera hämtningsdatum:
//!
//!   position 0-1: "h1" = varje vecka, "h2" = varannan vecka
//!   position 2:   "1" = udda veckor, "2" = jämna veckor
//!   position 3:   "1"=måndag, "2"=tisdag, ..., "7"=söndag
//!
//! Vi speglar widgetens logik: cachar hela datasetet 12 h i minnet,
//! filtrerar autocomplete som substring-match, och genererar
//! ~26 explicit anchor-datum vid schema-anrop.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{Datelike, Duration as ChronoDuration, NaiveDate, Weekday};
use serde::Deserialize;
use tokio::sync::RwLock;
use tokio::time::Instant;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const TTL: Duration = Duration::from_secs(12 * 3600);
const AUTOCOMPLETE_LIMIT: usize = 25;

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    /// URL-path på vattenmiljoresurs.se (t.ex. "harjedalen").
    pub path: &'static str,
    /// SiteVision-portlet-UUID (t.ex. "12.383d66bc198bb6c1bead0d").
    pub portlet_id: &'static str,
}

pub struct VattenMiljoResurs {
    http: reqwest::Client,
    cfg: Config,
    cache: RwLock<Option<Arc<Dataset>>>,
}

struct Dataset {
    fetched_at: Instant,
    entries: Vec<Entry>,
}

#[derive(Deserialize, Debug, Clone)]
struct RawResponse {
    #[serde(default, rename = "allAddresses")]
    all_addresses: Vec<RawEntry>,
}

#[derive(Deserialize, Debug, Clone)]
struct RawEntry {
    #[serde(default)]
    ort: String,
    #[serde(default)]
    hamtstalle: String,
    #[serde(default, rename = "tbfurenhkorlistarad_strtomningsfrekvenskod")]
    frekvenskod: String,
}

#[derive(Debug, Clone)]
struct Entry {
    ort: String,
    hamtstalle: String,
    frekvenskod: String,
}

impl VattenMiljoResurs {
    pub fn new(http: reqwest::Client, cfg: Config) -> Self {
        Self {
            http,
            cfg,
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
        let url = format!(
            "https://www.vattenmiljoresurs.se/{}?sv.target={pid}&sv.{pid}.route=/allAddresses&svAjaxReqParam=ajax",
            self.cfg.path,
            pid = self.cfg.portlet_id,
        );
        let resp: RawResponse = self
            .http
            .get(&url)
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Accept", "application/json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let entries = resp
            .all_addresses
            .into_iter()
            .filter_map(|e| {
                if e.hamtstalle.trim().is_empty() {
                    None
                } else {
                    Some(Entry {
                        ort: e.ort,
                        hamtstalle: e.hamtstalle,
                        frekvenskod: e.frekvenskod,
                    })
                }
            })
            .collect();
        let arc = Arc::new(Dataset {
            fetched_at: Instant::now(),
            entries,
        });
        *guard = Some(arc.clone());
        Ok(arc)
    }
}

#[async_trait]
impl Provider for VattenMiljoResurs {
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
        let q = query.trim().to_lowercase();
        if q.len() < 2 {
            return Ok(vec![]);
        }
        let ds = self.dataset().await?;
        let mut out = Vec::new();
        for e in &ds.entries {
            if out.len() >= AUTOCOMPLETE_LIMIT {
                break;
            }
            let hay = format!("{} {}", e.hamtstalle, e.ort).to_lowercase();
            if hay.contains(&q) {
                out.push(Suggestion {
                    value: format_suggestion(e),
                });
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let Some(kod) = extract_frekvenskod(address) else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };
        let Some(scheme) = decode_scheme(&kod) else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };
        let today = chrono::Local::now().date_naive();
        let dates = generate_dates(&scheme, today, 27);
        Ok(PickupSchedule {
            address: address.to_string(),
            series: vec![PickupSeries {
                waste_type: "Hushållsavfall".to_string(),
                frequency_text: format_scheme(&scheme),
                interval_weeks: None,
                anchor: dates,
            }],
        })
    }
}

fn format_suggestion(e: &Entry) -> String {
    let street = titlecase(&e.hamtstalle);
    let ort = titlecase(&e.ort);
    format!("{street}, {ort} ({})", e.frekvenskod)
}

fn titlecase(s: &str) -> String {
    s.split(' ')
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(c) => {
                    let head: String = c.to_uppercase().collect();
                    format!("{head}{}", chars.as_str())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_frekvenskod(s: &str) -> Option<String> {
    let open = s.rfind('(')?;
    let close = s.rfind(')')?;
    if close <= open {
        return None;
    }
    let inner = &s[open + 1..close];
    if inner.starts_with('h') && inner.len() >= 4 {
        Some(inner.to_string())
    } else {
        None
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Interval {
    Weekly,
    Biweekly,
}

#[derive(Debug, PartialEq, Eq)]
enum Parity {
    Odd,
    Even,
}

#[derive(Debug, PartialEq, Eq)]
struct Scheme {
    interval: Interval,
    parity: Parity,
    weekday: Weekday,
}

fn decode_scheme(kod: &str) -> Option<Scheme> {
    let bytes = kod.as_bytes();
    if bytes.len() < 4 {
        return None;
    }
    let interval = match &kod[0..2] {
        "h1" => Interval::Weekly,
        "h2" => Interval::Biweekly,
        _ => return None,
    };
    let parity = match bytes[2] {
        b'1' => Parity::Odd,
        b'2' => Parity::Even,
        _ => return None,
    };
    let weekday = match bytes[3] {
        b'1' => Weekday::Mon,
        b'2' => Weekday::Tue,
        b'3' => Weekday::Wed,
        b'4' => Weekday::Thu,
        b'5' => Weekday::Fri,
        b'6' => Weekday::Sat,
        b'7' => Weekday::Sun,
        _ => return None,
    };
    Some(Scheme {
        interval,
        parity,
        weekday,
    })
}

fn format_scheme(s: &Scheme) -> String {
    let interval = match s.interval {
        Interval::Weekly => "Varje vecka",
        Interval::Biweekly => "Varannan vecka",
    };
    let parity = match s.parity {
        Parity::Odd => "udda vecka",
        Parity::Even => "jämn vecka",
    };
    let day = weekday_sv(s.weekday);
    if matches!(s.interval, Interval::Weekly) {
        format!("{interval} — {day}")
    } else {
        format!("{interval} — {day} ({parity})")
    }
}

fn weekday_sv(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "måndag",
        Weekday::Tue => "tisdag",
        Weekday::Wed => "onsdag",
        Weekday::Thu => "torsdag",
        Weekday::Fri => "fredag",
        Weekday::Sat => "lördag",
        Weekday::Sun => "söndag",
    }
}

fn matches_scheme(date: NaiveDate, scheme: &Scheme) -> bool {
    if date.weekday() != scheme.weekday {
        return false;
    }
    match scheme.interval {
        Interval::Weekly => true,
        Interval::Biweekly => {
            let week = date.iso_week().week();
            match scheme.parity {
                Parity::Odd => week % 2 == 1,
                Parity::Even => week % 2 == 0,
            }
        }
    }
}

fn generate_dates(scheme: &Scheme, today: NaiveDate, count: usize) -> Vec<NaiveDate> {
    let target = scheme.weekday.num_days_from_monday() as i64;
    let current = today.weekday().num_days_from_monday() as i64;
    let days_forward = (target - current).rem_euclid(7);
    let mut date = today + ChronoDuration::days(days_forward);
    while !matches_scheme(date, scheme) {
        date += ChronoDuration::days(7);
    }
    let step = match scheme.interval {
        Interval::Weekly => 7,
        Interval::Biweekly => 14,
    };
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(date);
        date += ChronoDuration::days(step);
        if matches!(scheme.interval, Interval::Biweekly) && !matches_scheme(date, scheme) {
            date += ChronoDuration::days(7);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_scheme_biweekly_even_friday() {
        let s = decode_scheme("h225").unwrap();
        assert_eq!(s.interval, Interval::Biweekly);
        assert_eq!(s.parity, Parity::Even);
        assert_eq!(s.weekday, Weekday::Fri);
    }

    #[test]
    fn decode_scheme_weekly_odd_monday() {
        let s = decode_scheme("h111").unwrap();
        assert_eq!(s.interval, Interval::Weekly);
        assert_eq!(s.weekday, Weekday::Mon);
    }

    #[test]
    fn decode_scheme_rejects_invalid() {
        assert!(decode_scheme("").is_none());
        assert!(decode_scheme("h3").is_none());
        assert!(decode_scheme("h139").is_none());
        assert!(decode_scheme("h298").is_none());
    }

    #[test]
    fn extract_frekvenskod_from_suggestion() {
        assert_eq!(
            extract_frekvenskod("Sonfjällsgatan 12, Hede (h225)"),
            Some("h225".to_string())
        );
        assert!(extract_frekvenskod("Sonfjällsgatan 12, Hede").is_none());
        assert!(extract_frekvenskod("Storgatan 1 (extra info)").is_none());
    }

    #[test]
    fn format_scheme_weekly_has_no_parity() {
        let s = Scheme {
            interval: Interval::Weekly,
            parity: Parity::Odd,
            weekday: Weekday::Mon,
        };
        assert_eq!(format_scheme(&s), "Varje vecka — måndag");
    }

    #[test]
    fn format_scheme_biweekly_includes_parity() {
        let s = Scheme {
            interval: Interval::Biweekly,
            parity: Parity::Even,
            weekday: Weekday::Fri,
        };
        assert_eq!(format_scheme(&s), "Varannan vecka — fredag (jämn vecka)");
    }

    #[test]
    fn generate_dates_biweekly_even_friday() {
        // 2026-09-11 is Fri, week 37 (odd) → first even-week Fri is 2026-09-18
        // (week 38).
        let today = NaiveDate::from_ymd_opt(2026, 9, 12).unwrap();
        let s = Scheme {
            interval: Interval::Biweekly,
            parity: Parity::Even,
            weekday: Weekday::Fri,
        };
        let dates = generate_dates(&s, today, 5);
        assert_eq!(dates.len(), 5);
        assert_eq!(dates[0], NaiveDate::from_ymd_opt(2026, 9, 18).unwrap());
        for d in &dates {
            assert_eq!(d.weekday(), Weekday::Fri);
            assert_eq!(d.iso_week().week() % 2, 0);
        }
    }

    #[test]
    fn generate_dates_weekly_ignores_parity() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 12).unwrap();
        let s = Scheme {
            interval: Interval::Weekly,
            parity: Parity::Odd,
            weekday: Weekday::Mon,
        };
        let dates = generate_dates(&s, today, 4);
        assert_eq!(dates.len(), 4);
        assert_eq!(dates[0], NaiveDate::from_ymd_opt(2026, 9, 14).unwrap());
        for w in dates.windows(2) {
            assert_eq!((w[1] - w[0]).num_days(), 7);
        }
    }

    #[test]
    fn titlecase_capitalizes_word_starts() {
        assert_eq!(titlecase("sonfjällsgatan 12 a"), "Sonfjällsgatan 12 A");
        assert_eq!(titlecase("hede"), "Hede");
    }
}
