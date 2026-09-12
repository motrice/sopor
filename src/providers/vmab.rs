//! VMAB / Rabadang — publik jQuery + fullcalendar-widget som används
//! av VMAB (Bromölla, cal-bromolla.vmab.se) och Ronneby Miljöteknik
//! (kalender.fyrfackronneby.se). Två anonyma PHP-endpoints:
//!
//!   POST /search_suggestions.php  body `search_address=<query>`
//!     → HTML `<ul><li id="<pickupid>"><span class="address">STREET</span>
//!             <span class="city">CITY</span></li>...</ul>`
//!   POST /get_data.php  body `chosen_address=<addr>&chosen_address_pickupid=<id>`
//!     → HTML som innehåller en JS-array `events: [{ title: '<fraktion>',
//!                                                    start: 'YYYY-MM-DD' }, ...]`
//!       med 2+ års pickups per fraktion.
//!
//! Vi grupperar events efter title och emitterar explicit datum-serier
//! (ingen RRULE, servern har redan alla datum).

use async_trait::async_trait;
use chrono::NaiveDate;
use regex::Regex;
use reqwest::Client;
use std::collections::BTreeMap;
use std::sync::OnceLock;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    /// Base URL utan trailing slash, t.ex. `https://cal-bromolla.vmab.se`.
    pub base_url: &'static str,
}

pub struct Vmab {
    http: Client,
    cfg: Config,
}

impl Vmab {
    pub fn new(http: Client, cfg: Config) -> Self {
        Self { http, cfg }
    }
}

#[async_trait]
impl Provider for Vmab {
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
        let body = format!("search_address={}", urlencode(q));
        let html: String = self
            .http
            .post(format!("{}/search_suggestions.php", self.cfg.base_url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .text()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(parse_suggestions(&html))
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let Some((addr, id)) = split_suggestion(address) else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };
        let body = format!(
            "chosen_address={}&chosen_address_pickupid={}",
            urlencode(&addr),
            urlencode(&id)
        );
        let html: String = self
            .http
            .post(format!("{}/get_data.php", self.cfg.base_url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .text()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(build_schedule(address.to_string(), &html))
    }
}

fn urlencode(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}

fn parse_suggestions(html: &str) -> Vec<Suggestion> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(
            r#"(?s)<li[^>]*id="(\d+)"[^>]*>\s*<span[^>]*class="address"[^>]*>([^<]+)</span>\s*<span[^>]*class="city"[^>]*>([^<]+)</span>"#,
        )
        .unwrap()
    });
    let mut seen = std::collections::HashSet::new();
    re.captures_iter(html)
        .filter_map(|c| {
            let id = c.get(1)?.as_str().trim();
            let addr = c.get(2)?.as_str().trim();
            let city = c.get(3)?.as_str().trim();
            let value = format!("{addr}, {city} (id={id})");
            if !seen.insert(value.clone()) {
                return None;
            }
            Some(Suggestion { value })
        })
        .collect()
}

fn split_suggestion(s: &str) -> Option<(String, String)> {
    let open = s.rfind('(')?;
    let close = s.rfind(')')?;
    if close <= open {
        return None;
    }
    let inner = &s[open + 1..close];
    let id = inner.strip_prefix("id=")?.to_string();
    // Address part = "Storgatan 45, Bromölla" — VMAB's get_data expects the
    // street portion only (no city).
    let head = s[..open].trim().trim_end_matches(',').trim();
    let addr = head.rsplit_once(',').map(|(a, _)| a.trim().to_string()).unwrap_or(head.to_string());
    Some((addr, id))
}

fn build_schedule(address: String, html: &str) -> PickupSchedule {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"title:\s*'([^']+)',\s*start:\s*'(\d{4}-\d{2}-\d{2})'").unwrap()
    });
    let mut grouped: BTreeMap<String, Vec<NaiveDate>> = BTreeMap::new();
    for c in re.captures_iter(html) {
        let title = c.get(1).unwrap().as_str().to_string();
        let Some(date) = NaiveDate::parse_from_str(c.get(2).unwrap().as_str(), "%Y-%m-%d").ok()
        else {
            continue;
        };
        let list = grouped.entry(title).or_default();
        if !list.contains(&date) {
            list.push(date);
        }
    }
    let series = grouped
        .into_iter()
        .map(|(waste_type, mut anchor)| {
            anchor.sort();
            PickupSeries {
                waste_type,
                frequency_text: "Ordinarie hämtningsdag".to_string(),
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
    fn parse_suggestions_extracts_id_addr_city() {
        let html = r#"<ul><li class="selected" id="2000024"><span class="address">Storgatan 45</span> <span class="city">Bromölla</span></li><li class="" id="2000028"><span class="address">Storgatan 1</span> <span class="city">Bromölla</span></li></ul>"#;
        let s = parse_suggestions(html);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].value, "Storgatan 45, Bromölla (id=2000024)");
        assert_eq!(s[1].value, "Storgatan 1, Bromölla (id=2000028)");
    }

    #[test]
    fn split_suggestion_extracts_addr_and_id() {
        let (addr, id) = split_suggestion("Storgatan 45, Bromölla (id=2000024)").unwrap();
        assert_eq!(addr, "Storgatan 45");
        assert_eq!(id, "2000024");
        assert!(split_suggestion("Storgatan 45").is_none());
    }

    #[test]
    fn build_schedule_groups_events_by_title() {
        let html = r#"
            events: [
                { title: 'Matavfall, 140 l kärl', start: '2026-01-05' },
                { title: 'Matavfall, 140 l kärl', start: '2026-01-19' },
                { title: 'Restavfall, 190 l kärl', start: '2026-01-12' },
            ]"#;
        let s = build_schedule("addr".into(), html);
        assert_eq!(s.series.len(), 2);
        let mat = s.series.iter().find(|p| p.waste_type.starts_with("Matavfall")).unwrap();
        assert_eq!(mat.anchor.len(), 2);
        assert_eq!(mat.anchor[0], NaiveDate::from_ymd_opt(2026, 1, 5).unwrap());
    }
}
