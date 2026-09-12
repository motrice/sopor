//! Sörmland Vatten & Avfall — sopschema för Katrineholm, Vingåker, Flen.
//!
//! Widgeten på `sormlandvatten.se/avfall-atervinning/hushallsavfall/sophamtning/`
//! anropar WordPress admin-ajax med en nonce som byts ~24h:
//!
//!   POST /wp-admin/admin-ajax.php
//!   Content-Type: application/x-www-form-urlencoded
//!   action=garbage_collection_search&nonce=<nonce>&keyword=<q>
//!   &attachment_ids=<id>&limit=50
//!
//! Svar: `{success:true, data:{searched_data: [{id, address, city, day}]}}`
//! där `day` t.ex. "Fredag jämn vecka" — vi har redan tillräckligt för
//! att generera datum lokalt utan separat schedule-anrop.
//!
//! Nonce och attachment_ids skrapas från själva sophämtnings-sidan och
//! cachas 12 h. Om servern svarar `success:false` (nonce har roterat)
//! försöker vi hämta en ny och skickar om requesten.

use std::collections::HashSet;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{Datelike, Duration as ChronoDuration, NaiveDate, Weekday};
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;
use tokio::time::Instant;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const PAGE_URL: &str =
    "https://sormlandvatten.se/avfall-atervinning/hushallsavfall/sophamtning/";
const AJAX_URL: &str = "https://sormlandvatten.se/wp-admin/admin-ajax.php";
const NONCE_TTL: Duration = Duration::from_secs(12 * 3600);
const AUTOCOMPLETE_LIMIT: usize = 25;

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub cities: &'static [&'static str],
}

pub struct SormlandVatten {
    http: Client,
    cfg: Config,
    nonce: RwLock<Option<CachedNonce>>,
}

#[derive(Clone)]
struct CachedNonce {
    nonce: String,
    attachment_ids: String,
    fetched_at: Instant,
}

impl SormlandVatten {
    pub fn new(http: Client, cfg: Config) -> Self {
        Self {
            http,
            cfg,
            nonce: RwLock::new(None),
        }
    }

    fn matches_city(&self, city: &str) -> bool {
        let needle = city.trim().to_lowercase();
        self.cfg
            .cities
            .iter()
            .any(|c| c.to_lowercase() == needle)
    }

    async fn nonce(&self, force_refresh: bool) -> Result<CachedNonce, ProviderError> {
        if !force_refresh {
            if let Some(n) = self.nonce.read().await.as_ref() {
                if n.fetched_at.elapsed() < NONCE_TTL {
                    return Ok(n.clone());
                }
            }
        }
        let mut guard = self.nonce.write().await;
        if !force_refresh {
            if let Some(n) = guard.as_ref() {
                if n.fetched_at.elapsed() < NONCE_TTL {
                    return Ok(n.clone());
                }
            }
        }
        let html = self
            .http
            .get(PAGE_URL)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let (nonce, attachment_ids) = extract_nonce(&html)
            .ok_or_else(|| ProviderError("could not extract nonce from page".to_string()))?;
        let entry = CachedNonce {
            nonce,
            attachment_ids,
            fetched_at: Instant::now(),
        };
        *guard = Some(entry.clone());
        Ok(entry)
    }

    async fn search(&self, query: &str) -> Result<Vec<Hit>, ProviderError> {
        for attempt in 0..2 {
            let n = self.nonce(attempt > 0).await?;
            let body = format!(
                "action=garbage_collection_search&nonce={}&keyword={}&attachment_ids={}&limit=50",
                urlencode(&n.nonce),
                urlencode(query),
                urlencode(&n.attachment_ids),
            );
            let resp: Response = self
                .http
                .post(AJAX_URL)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("X-Requested-With", "XMLHttpRequest")
                .header("Referer", PAGE_URL)
                .body(body)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            if resp.success {
                return Ok(resp.data.map(|d| d.searched_data).unwrap_or_default());
            }
            // Nonce expired → retry once with a fresh one.
        }
        Ok(vec![])
    }
}

fn urlencode(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}

fn extract_nonce(html: &str) -> Option<(String, String)> {
    let nonce = Regex::new(r#"garbage_collection_search_nonce["'][^>]{0,60}value=["'](\w+)"#)
        .unwrap()
        .captures(html)?
        .get(1)?
        .as_str()
        .to_string();
    let attachment = Regex::new(r#"garbage_collection_attachment_ids["'][^>]{0,60}value=["'](\w+)"#)
        .unwrap()
        .captures(html)?
        .get(1)?
        .as_str()
        .to_string();
    Some((nonce, attachment))
}

#[derive(Debug, Deserialize)]
struct Response {
    success: bool,
    #[serde(default)]
    data: Option<Data>,
}

#[derive(Debug, Deserialize)]
struct Data {
    #[serde(default)]
    searched_data: Vec<Hit>,
}

#[derive(Debug, Deserialize, Clone)]
struct Hit {
    #[serde(default)]
    address: Option<String>,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    day: Option<String>,
}

#[async_trait]
impl Provider for SormlandVatten {
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
        "Sophämtningsdata från Sörmland Vatten & Avfall AB (SVAAB). \
         Widget-nonce förnyas automatiskt när den föråldras."
    }

    async fn autocomplete(&self, query: &str) -> Result<Vec<Suggestion>, ProviderError> {
        let q = query.trim();
        if q.len() < 2 {
            return Ok(vec![]);
        }
        let hits = self.search(q).await?;
        let mut seen = HashSet::new();
        Ok(hits
            .into_iter()
            .filter_map(|h| {
                let city = h.city.clone().unwrap_or_default();
                if !self.matches_city(&city) {
                    return None;
                }
                let value = format_suggestion(&h)?;
                if !seen.insert(value.clone()) {
                    return None;
                }
                Some(Suggestion { value })
            })
            .take(AUTOCOMPLETE_LIMIT)
            .collect())
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let Some((display, scheme)) = parse_suggestion(address) else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };
        let today = chrono::Local::now().date_naive();
        let anchor = generate_dates(&scheme, today, 27);
        let series = vec![PickupSeries {
            waste_type: "Hushållsavfall".to_string(),
            frequency_text: describe_scheme(&scheme),
            interval_weeks: None,
            anchor,
        }];
        Ok(PickupSchedule {
            address: display,
            series,
        })
    }
}

fn format_suggestion(h: &Hit) -> Option<String> {
    let addr = h.address.as_deref()?.trim();
    let city = h.city.as_deref()?.trim();
    let day = h.day.as_deref()?.trim();
    if addr.is_empty() || city.is_empty() || day.is_empty() {
        return None;
    }
    Some(format!("{addr}, {} — {day}", titlecase(city)))
}

fn titlecase(s: &str) -> String {
    s.chars()
        .enumerate()
        .map(|(i, c)| {
            if i == 0 {
                c.to_uppercase().next().unwrap_or(c)
            } else {
                c.to_lowercase().next().unwrap_or(c)
            }
        })
        .collect()
}

fn parse_suggestion(s: &str) -> Option<(String, Scheme)> {
    // "Storgatan 16, Flen — Fredag jämn vecka"
    let (display, day_part) = s.rsplit_once(" — ")?;
    let scheme = parse_day(day_part.trim())?;
    Some((display.to_string(), scheme))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Interval {
    Weekly,
    Biweekly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Parity {
    Odd,
    Even,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Scheme {
    weekday: Weekday,
    interval: Interval,
    parity: Parity,
}

fn parse_day(s: &str) -> Option<Scheme> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    let weekday = match parts[0].to_lowercase().as_str() {
        "måndag" => Weekday::Mon,
        "tisdag" => Weekday::Tue,
        "onsdag" => Weekday::Wed,
        "torsdag" => Weekday::Thu,
        "fredag" => Weekday::Fri,
        "lördag" => Weekday::Sat,
        "söndag" => Weekday::Sun,
        _ => return None,
    };
    // Format: "<Weekday> <udda|jämn|varje> vecka"
    let modifier = parts[1].to_lowercase();
    let (interval, parity) = match modifier.as_str() {
        "varje" => (Interval::Weekly, Parity::Odd), // parity ignored for weekly
        "udda" => (Interval::Biweekly, Parity::Odd),
        "jämn" | "jamn" => (Interval::Biweekly, Parity::Even),
        _ => return None,
    };
    Some(Scheme {
        weekday,
        interval,
        parity,
    })
}

fn describe_scheme(s: &Scheme) -> String {
    let wd = match s.weekday {
        Weekday::Mon => "måndag",
        Weekday::Tue => "tisdag",
        Weekday::Wed => "onsdag",
        Weekday::Thu => "torsdag",
        Weekday::Fri => "fredag",
        Weekday::Sat => "lördag",
        Weekday::Sun => "söndag",
    };
    match s.interval {
        Interval::Weekly => format!("Varje vecka — {wd}"),
        Interval::Biweekly => {
            let p = match s.parity {
                Parity::Odd => "udda",
                Parity::Even => "jämn",
            };
            format!("Varannan vecka — {wd} ({p} vecka)")
        }
    }
}

fn matches_parity(date: NaiveDate, parity: Parity) -> bool {
    let week = date.iso_week().week();
    match parity {
        Parity::Odd => week % 2 == 1,
        Parity::Even => week % 2 == 0,
    }
}

fn generate_dates(scheme: &Scheme, today: NaiveDate, count: usize) -> Vec<NaiveDate> {
    let target = scheme.weekday.num_days_from_monday() as i64;
    let current = today.weekday().num_days_from_monday() as i64;
    let days_forward = (target - current).rem_euclid(7);
    let mut date = today + ChronoDuration::days(days_forward);
    if matches!(scheme.interval, Interval::Biweekly) {
        while !matches_parity(date, scheme.parity) {
            date += ChronoDuration::days(7);
        }
    }
    let step = match scheme.interval {
        Interval::Weekly => 7,
        Interval::Biweekly => 14,
    };
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(date);
        date += ChronoDuration::days(step);
        if matches!(scheme.interval, Interval::Biweekly) && !matches_parity(date, scheme.parity) {
            date += ChronoDuration::days(7);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_nonce_from_page_html() {
        let html = r#"
            <input type="hidden" name="garbage_collection_search_nonce" value="ab5b0b714f">
            <input type="hidden" name="garbage_collection_attachment_ids" value="18483">
        "#;
        let (nonce, ids) = extract_nonce(html).unwrap();
        assert_eq!(nonce, "ab5b0b714f");
        assert_eq!(ids, "18483");
    }

    #[test]
    fn parse_day_variants() {
        let s = parse_day("Fredag jämn vecka").unwrap();
        assert_eq!(s.weekday, Weekday::Fri);
        assert_eq!(s.interval, Interval::Biweekly);
        assert_eq!(s.parity, Parity::Even);

        let s = parse_day("Måndag udda vecka").unwrap();
        assert_eq!(s.weekday, Weekday::Mon);
        assert_eq!(s.parity, Parity::Odd);

        let s = parse_day("Torsdag varje vecka").unwrap();
        assert_eq!(s.weekday, Weekday::Thu);
        assert_eq!(s.interval, Interval::Weekly);

        assert!(parse_day("Weekly Monday").is_none());
    }

    #[test]
    fn format_suggestion_includes_day() {
        let h = Hit {
            address: Some("Storgatan 16".into()),
            city: Some("FLEN".into()),
            day: Some("Fredag jämn vecka".into()),
        };
        assert_eq!(
            format_suggestion(&h).unwrap(),
            "Storgatan 16, Flen — Fredag jämn vecka"
        );
    }

    #[test]
    fn parse_suggestion_round_trip() {
        let (display, scheme) = parse_suggestion("Storgatan 16, Flen — Fredag jämn vecka").unwrap();
        assert_eq!(display, "Storgatan 16, Flen");
        assert_eq!(scheme.weekday, Weekday::Fri);
        assert_eq!(scheme.interval, Interval::Biweekly);
        assert_eq!(scheme.parity, Parity::Even);
    }

    #[test]
    fn generate_dates_biweekly_even_friday() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 12).unwrap();
        let scheme = Scheme {
            weekday: Weekday::Fri,
            interval: Interval::Biweekly,
            parity: Parity::Even,
        };
        let d = generate_dates(&scheme, today, 5);
        assert_eq!(d[0], NaiveDate::from_ymd_opt(2026, 9, 18).unwrap());
        for w in d.windows(2) {
            assert_eq!((w[1] - w[0]).num_days(), 14);
        }
    }

    #[test]
    fn generate_dates_weekly_ignores_parity() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 12).unwrap();
        let scheme = Scheme {
            weekday: Weekday::Mon,
            interval: Interval::Weekly,
            parity: Parity::Odd,
        };
        let d = generate_dates(&scheme, today, 4);
        assert_eq!(d[0], NaiveDate::from_ymd_opt(2026, 9, 14).unwrap());
        for w in d.windows(2) {
            assert_eq!((w[1] - w[0]).num_days(), 7);
        }
    }

    #[test]
    fn describe_scheme_readable() {
        let s = Scheme {
            weekday: Weekday::Fri,
            interval: Interval::Biweekly,
            parity: Parity::Even,
        };
        assert_eq!(describe_scheme(&s), "Varannan vecka — fredag (jämn vecka)");
    }
}
