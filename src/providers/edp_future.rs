//! Provider for kommuner using EDP Future / FutureWeb / SimpleWastePickup
//! (VertiGIS, formerly EDP Konsult). Public JSON API, no auth. Each kommun
//! is configured with the `api_url` of its FutureWeb instance plus an
//! optional `cities` allow-list when several kommuner share the same
//! backend (SSAM, Roslagsvatten, Kretslopp Sydost, Vafab Miljö, …).
//!
//! Endpoints:
//!   POST {api_url}/SearchAdress?searchText=<street>
//!     -> { "Succeeded": bool, "Buildings": [string, ...] }
//!     Buildings are strings of the form "Storgatan 1, ORT (123456)".
//!   GET  {api_url}/GetWastePickupSchedule?address=<urlencoded Building>
//!     -> { "RhServices": [{ WasteType, NextWastePickup,
//!                           WastePickupsPerYear, WastePickupFrequency,
//!                           BinType, ... }, ...] }

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
    /// Trailing slash will be stripped if present.
    pub api_url: &'static str,
    /// `None` = no filter (bolag/route only covers one kommun).
    /// `Some(&[...])` = only show suggestions whose Building locality
    /// matches one of these names (case-insensitive).
    pub cities: Option<&'static [&'static str]>,
}

pub struct EdpFuture {
    http: reqwest::Client,
    cfg: Config,
}

impl EdpFuture {
    pub fn new(http: reqwest::Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    fn api(&self) -> &str {
        self.cfg.api_url.trim_end_matches('/')
    }

    async fn search_address(&self, query: &str) -> Result<Vec<String>, ProviderError> {
        let url = format!("{}/SearchAdress", self.api());
        // The upstream IIS deployments return HTTP 411 unless an explicit
        // Content-Length header is set on the POST. reqwest uses chunked
        // transfer encoding by default for empty bodies, which IIS rejects.
        let resp: SearchResp = self
            .http
            .post(&url)
            .query(&[("searchText", query)])
            .header(reqwest::header::CONTENT_LENGTH, "0")
            .body("")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if !resp.succeeded {
            return Ok(vec![]);
        }
        Ok(resp.buildings)
    }

    async fn get_schedule(&self, address: &str) -> Result<String, ProviderError> {
        let url = format!("{}/GetWastePickupSchedule", self.api());
        let text = self
            .http
            .get(&url)
            .query(&[("address", address)])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok(text)
    }

    fn matches_city(&self, city: &str) -> bool {
        match self.cfg.cities {
            None => true,
            Some(list) => {
                let needle = city.to_lowercase();
                list.iter().any(|c| c.to_lowercase() == needle)
            }
        }
    }
}

#[derive(Deserialize)]
struct SearchResp {
    #[serde(rename = "Succeeded")]
    succeeded: bool,
    #[serde(rename = "Buildings", default)]
    buildings: Vec<String>,
}

#[derive(Deserialize)]
struct ScheduleResp {
    #[serde(rename = "RhServices", default)]
    rh_services: Vec<RhService>,
}

#[derive(Deserialize)]
struct RhService {
    #[serde(rename = "WasteType", default)]
    waste_type: String,
    #[serde(rename = "NextWastePickup", default)]
    next_waste_pickup: String,
    #[serde(rename = "WastePickupsPerYear", default)]
    wastepickups_per_year: Option<i32>,
    #[serde(rename = "WastePickupFrequency", default)]
    waste_pickup_frequency: String,
}

#[derive(Debug, Clone, PartialEq)]
struct Building {
    full: String,
    city: String,
}

fn parse_building(s: &str) -> Option<Building> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Drop a trailing " (id)" if present so we can pull out the city, but
    // keep it on `full` since the schedule endpoint round-trips the whole
    // string verbatim.
    let without_id = match (trimmed.rfind(" ("), trimmed.ends_with(')')) {
        (Some(idx), true) => &trimmed[..idx],
        _ => trimmed,
    };
    let city = without_id.rsplit(',').next().unwrap_or("").trim();
    if city.is_empty() {
        return None;
    }
    Some(Building {
        full: trimmed.to_string(),
        city: city.to_string(),
    })
}

#[async_trait]
impl Provider for EdpFuture {
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
        let buildings = self.search_address(q).await?;
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for raw in buildings {
            let Some(b) = parse_building(&raw) else {
                continue;
            };
            if !self.matches_city(&b.city) {
                continue;
            }
            if seen.insert(b.full.clone()) {
                out.push(Suggestion { value: b.full });
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        // Most of the address-input strings come back from our own autocomplete
        // and round-trip verbatim. Try a direct lookup first; if it returns
        // nothing, re-query upstream and find the matching Building.
        let resolved = if !address.contains('(') {
            let street = address.split(',').next().unwrap_or(address).trim();
            let buildings = self.search_address(street).await?;
            buildings
                .into_iter()
                .find(|b| {
                    let Some(parsed) = parse_building(b) else {
                        return false;
                    };
                    self.matches_city(&parsed.city)
                        && (parsed.full.eq_ignore_ascii_case(address)
                            || strip_id(&parsed.full).eq_ignore_ascii_case(address))
                })
        } else {
            Some(address.to_string())
        };

        let Some(building) = resolved else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };

        let json = self.get_schedule(&building).await?;
        let series = parse_schedule_response(&json);
        Ok(PickupSchedule {
            address: building,
            series,
        })
    }
}

fn strip_id(s: &str) -> &str {
    match (s.rfind(" ("), s.ends_with(')')) {
        (Some(idx), true) => &s[..idx],
        _ => s,
    }
}

fn parse_schedule_response(json: &str) -> Vec<PickupSeries> {
    let Ok(parsed) = serde_json::from_str::<ScheduleResp>(json) else {
        return Vec::new();
    };

    // (waste_type) -> (frequency_text, per_year, dates)
    let mut by_type: BTreeMap<String, (String, Option<i32>, Vec<NaiveDate>)> = BTreeMap::new();
    for svc in parsed.rh_services {
        let Some(date) = parse_pickup_date(&svc.next_waste_pickup) else {
            continue;
        };
        if svc.waste_type.is_empty() {
            continue;
        }
        let entry = by_type.entry(svc.waste_type.clone()).or_insert_with(|| {
            (
                svc.waste_pickup_frequency.clone(),
                svc.wastepickups_per_year,
                Vec::new(),
            )
        });
        if entry.0.is_empty() {
            entry.0 = svc.waste_pickup_frequency;
        }
        if entry.1.is_none() {
            entry.1 = svc.wastepickups_per_year;
        }
        entry.2.push(date);
    }

    let mut series = Vec::new();
    for (waste_type, (frequency_text, per_year, mut dates)) in by_type {
        dates.sort();
        dates.dedup();
        let interval_weeks = interval_from_pickups_per_year(per_year)
            .or_else(|| interval_from_frequency_text(&frequency_text));
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

fn parse_pickup_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // ISO format: "2026-06-30"
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d);
    }
    // Week format: "v25 Jun 2026" → ISO week (Monday).
    if let Some(rest) = s.strip_prefix('v').or_else(|| s.strip_prefix('V')) {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() == 3 {
            if let (Ok(week), Some(_month), Ok(year)) = (
                parts[0].parse::<u32>(),
                swedish_month_abbr_to_num(parts[1]),
                parts[2].parse::<i32>(),
            ) {
                if let Some(d) = NaiveDate::from_isoywd_opt(year, week, chrono::Weekday::Mon) {
                    return Some(d);
                }
            }
        }
    }
    // Month-only fallback: "Jun 2026" → day 1.
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() == 2 {
        if let (Some(month), Ok(year)) = (
            swedish_month_abbr_to_num(parts[0]),
            parts[1].parse::<i32>(),
        ) {
            return NaiveDate::from_ymd_opt(year, month, 1);
        }
    }
    None
}

fn swedish_month_abbr_to_num(s: &str) -> Option<u32> {
    match s.to_lowercase().as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "maj" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "okt" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

fn interval_from_pickups_per_year(per_year: Option<i32>) -> Option<u32> {
    match per_year? {
        52 => Some(1),
        26 => Some(2),
        13 => Some(4),
        // Higher cadences (3 ggr/vecka = 156, every weekday etc.) can't be
        // RRULE-projected with a single weekly anchor without missing
        // alternate days. Emit single date instead.
        n if n > 52 => None,
        _ => None,
    }
}

fn interval_from_frequency_text(freq: &str) -> Option<u32> {
    let lower = freq.to_lowercase();
    // Multiple weekdays per week ("Måndag, torsdag varje vecka",
    // "Varje vecka, Måndag Onsdag Fredag") shouldn't RRULE-project.
    if lower.contains(',') {
        return None;
    }
    if lower.contains("varje vecka") {
        return Some(1);
    }
    if lower.contains("varannan vecka") {
        return Some(2);
    }
    // "Var 4:e vecka", "Var 8:e vecka", …
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(cities: Option<&'static [&'static str]>) -> Config {
        Config {
            id: "test",
            name: "Test",
            placeholder: "",
            note: "",
            api_url: "https://example.invalid/FutureWeb/SimpleWastePickup",
            cities,
        }
    }

    #[test]
    fn parse_building_with_id() {
        let b = parse_building("Frögatan 76 -150, SKELLEFTEÅ (133427)").unwrap();
        assert_eq!(b.full, "Frögatan 76 -150, SKELLEFTEÅ (133427)");
        assert_eq!(b.city, "SKELLEFTEÅ");
    }

    #[test]
    fn parse_building_without_id() {
        let b = parse_building("Storgatan 1, Vollsjö").unwrap();
        assert_eq!(b.city, "Vollsjö");
    }

    #[test]
    fn parse_building_drops_empty() {
        assert!(parse_building("").is_none());
        assert!(parse_building("   ").is_none());
    }

    #[test]
    fn pickup_date_iso() {
        assert_eq!(
            parse_pickup_date("2026-06-30"),
            Some(NaiveDate::from_ymd_opt(2026, 6, 30).unwrap())
        );
    }

    #[test]
    fn pickup_date_week_format() {
        // ISO week 25 of 2026 = Monday 15 June 2026.
        assert_eq!(
            parse_pickup_date("v25 Jun 2026"),
            Some(NaiveDate::from_isoywd_opt(2026, 25, chrono::Weekday::Mon).unwrap())
        );
    }

    #[test]
    fn pickup_date_month_only() {
        assert_eq!(
            parse_pickup_date("Jun 2026"),
            Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap())
        );
    }

    #[test]
    fn pickup_date_empty_or_garbage() {
        assert_eq!(parse_pickup_date(""), None);
        assert_eq!(parse_pickup_date("vad som helst"), None);
    }

    #[test]
    fn interval_from_per_year_known_cadences() {
        assert_eq!(interval_from_pickups_per_year(Some(52)), Some(1));
        assert_eq!(interval_from_pickups_per_year(Some(26)), Some(2));
        assert_eq!(interval_from_pickups_per_year(Some(13)), Some(4));
        assert_eq!(interval_from_pickups_per_year(Some(156)), None);
        assert_eq!(interval_from_pickups_per_year(Some(7)), None);
        assert_eq!(interval_from_pickups_per_year(None), None);
    }

    #[test]
    fn interval_from_text_handles_swedish_variants() {
        assert_eq!(interval_from_frequency_text("Torsdag varje vecka"), Some(1));
        assert_eq!(interval_from_frequency_text("Tisdag varannan vecka"), Some(2));
        assert_eq!(interval_from_frequency_text("Var 4:e vecka"), Some(4));
        // Multi-day-per-week → None
        assert_eq!(
            interval_from_frequency_text("Måndag, torsdag varje vecka"),
            None
        );
        assert_eq!(
            interval_from_frequency_text("Varje vecka, Måndag Onsdag Fredag"),
            None
        );
        assert_eq!(interval_from_frequency_text("Varannan månad"), None);
    }

    #[test]
    fn parse_schedule_groups_by_waste_type_and_picks_interval() {
        let json = r#"{"RhServices":[
            {"WasteType":"Restavfall","NextWastePickup":"2026-06-18",
             "WastePickupsPerYear":52,"WastePickupFrequency":"Torsdag varje vecka"},
            {"WasteType":"Restavfall","NextWastePickup":"2026-06-18",
             "WastePickupsPerYear":52,"WastePickupFrequency":"Torsdag varje vecka"},
            {"WasteType":"Matavfall","NextWastePickup":"2026-06-25",
             "WastePickupsPerYear":26,"WastePickupFrequency":"Torsdag jämn vecka"}
        ]}"#;
        let series = parse_schedule_response(json);
        assert_eq!(series.len(), 2);
        let rest = series.iter().find(|s| s.waste_type == "Restavfall").unwrap();
        assert_eq!(rest.interval_weeks, Some(1));
        assert_eq!(rest.anchor.len(), 1);
        assert_eq!(
            rest.anchor[0],
            NaiveDate::from_ymd_opt(2026, 6, 18).unwrap()
        );
        let mat = series.iter().find(|s| s.waste_type == "Matavfall").unwrap();
        assert_eq!(mat.interval_weeks, Some(2));
    }

    #[test]
    fn parse_schedule_handles_multiday_weekly() {
        let json = r#"{"RhServices":[
            {"WasteType":"Brännbart","NextWastePickup":"2026-06-17",
             "WastePickupsPerYear":156,
             "WastePickupFrequency":"Varje vecka, Måndag Onsdag Fredag"}
        ]}"#;
        let series = parse_schedule_response(json);
        assert_eq!(series.len(), 1);
        // No RRULE; explicit single date.
        assert_eq!(series[0].interval_weeks, None);
        assert_eq!(series[0].anchor.len(), 1);
    }

    #[test]
    fn parse_schedule_drops_entries_with_no_date() {
        let json = r#"{"RhServices":[
            {"WasteType":"Restavfall","NextWastePickup":"","WastePickupsPerYear":52,
             "WastePickupFrequency":"Varje vecka"}
        ]}"#;
        assert!(parse_schedule_response(json).is_empty());
    }

    #[test]
    fn city_filter_allow_list() {
        let p = EdpFuture::new(
            reqwest::Client::new(),
            cfg(Some(&["VÄXJÖ", "Älmhult"])),
        );
        // Non-ASCII case folding must work for Swedish characters.
        assert!(p.matches_city("Växjö"));
        assert!(p.matches_city("ÄLMHULT"));
        assert!(p.matches_city("älmhult"));
        assert!(!p.matches_city("Lessebo"));
    }

    #[test]
    fn city_filter_none_matches_everything() {
        let p = EdpFuture::new(reqwest::Client::new(), cfg(None));
        assert!(p.matches_city("Anywhere"));
    }

    #[test]
    fn strip_id_helper() {
        assert_eq!(strip_id("X, ORT (123)"), "X, ORT");
        assert_eq!(strip_id("X, ORT"), "X, ORT");
    }
}
