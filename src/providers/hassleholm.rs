//! Hässleholm — Hässleholm Miljö AB (HMAB), SiteVision site backed by
//! Appbolaget's "universal waste" API (api-universal.appbolaget.se).
//!
//! Address autocomplete hits the Appbolaget API directly (open GET
//! JSON, no auth):
//!   GET /@universal/waste/addresses/search/?unit=<tenant>&query=<q>
//!     -> { data: [{ uuid, alias, address, city, designation }] }
//!
//! The month schedule has no public Appbolaget endpoint; it is served
//! by the `hsm.recycling-calendar` SiteVision webapp on the kommun's
//! own site via its /getmonth JSON route:
//!   GET <calendar page>?sv.target=<portlet>&sv.<portlet>.route=/getmonth
//!       &date=YYYY-MM-01&alias=<alias>&customer=&svAjaxReqParam=ajax
//!     -> { calendarMonth: { days: [{ date, currentMonth, services }] } }
//! The X-Requested-With: XMLHttpRequest header is required — without it
//! SiteVision renders the full HTML page instead of JSON.
//!
//! Upstream only publishes the near-term schedule (the current
//! "tömningskalender" period), so we emit explicit dates and rely on
//! the subscription refresh to pick up new months as they appear.
//! The PDF export on the same API is date-shifted -1 day (UTC
//! serialization bug upstream); the widget dates are what users see,
//! so /getmonth is authoritative.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{Datelike, NaiveDate, Utc};
use serde::Deserialize;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const SEARCH_URL: &str = "https://api-universal.appbolaget.se/@universal/waste/addresses/search/";

// Current month plus two ahead. Upstream data rarely extends further,
// and each month is a separate upstream request.
const MONTHS: u32 = 3;

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    /// Appbolaget tenant uuid (the `unit` query parameter).
    pub unit: &'static str,
    /// Public calendar page hosting the recycling-calendar webapp.
    pub calendar_url: &'static str,
    /// SiteVision portlet id of the hsm.recycling-calendar webapp.
    pub portlet_id: &'static str,
}

pub struct Hassleholm {
    http: reqwest::Client,
    cfg: Config,
}

impl Hassleholm {
    pub fn new(http: reqwest::Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    async fn search(&self, query: &str) -> Result<Vec<SearchHit>, ProviderError> {
        let resp = self
            .http
            .get(SEARCH_URL)
            .query(&[("unit", self.cfg.unit), ("query", query)])
            .send()
            .await?;
        if !resp.status().is_success() {
            return Ok(vec![]);
        }
        let body = resp.text().await?;
        parse_search(&body)
    }

    async fn fetch_month(&self, alias: &str, date: NaiveDate) -> Result<String, ProviderError> {
        let route_key = format!("sv.{}.route", self.cfg.portlet_id);
        let resp = self
            .http
            .get(self.cfg.calendar_url)
            .query(&[
                ("sv.target", self.cfg.portlet_id),
                (&route_key, "/getmonth"),
                ("date", &date.format("%Y-%m-%d").to_string()),
                ("alias", alias),
                ("customer", ""),
                ("svAjaxReqParam", "ajax"),
            ])
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await?;
        Ok(resp.text().await?)
    }
}

#[derive(Deserialize)]
struct SearchResponse {
    #[serde(default)]
    data: Vec<SearchHit>,
}

#[derive(Deserialize)]
struct SearchHit {
    address: String,
    city: String,
    alias: String,
}

#[derive(Deserialize)]
struct MonthResponse {
    #[serde(rename = "calendarMonth")]
    calendar_month: Option<CalendarMonth>,
}

#[derive(Deserialize)]
struct CalendarMonth {
    #[serde(default)]
    days: Vec<Day>,
}

#[derive(Deserialize)]
struct Day {
    date: String,
    #[serde(default, rename = "currentMonth")]
    current_month: bool,
    #[serde(default)]
    services: Option<Vec<Service>>,
}

#[derive(Deserialize)]
struct Service {
    name: String,
}

fn parse_search(json: &str) -> Result<Vec<SearchHit>, ProviderError> {
    let resp: SearchResponse = serde_json::from_str(json)?;
    Ok(resp.data)
}

fn display_address(hit: &SearchHit) -> String {
    format!("{}, {}", hit.address, hit.city)
}

/// Extract (date, waste type) pairs from one /getmonth response.
/// Adjacent-month filler days have `currentMonth: false` and are
/// skipped so months never double-count across requests.
fn parse_month(json: &str) -> Result<Vec<(NaiveDate, String)>, ProviderError> {
    let resp: MonthResponse = serde_json::from_str(json)?;
    let mut out = Vec::new();
    let Some(month) = resp.calendar_month else {
        return Ok(out);
    };
    for day in month.days {
        if !day.current_month {
            continue;
        }
        let Ok(date) = NaiveDate::parse_from_str(&day.date, "%Y-%m-%d") else {
            continue;
        };
        for service in day.services.unwrap_or_default() {
            out.push((date, service.name));
        }
    }
    Ok(out)
}

fn build_schedule(address: &str, pickups: Vec<(NaiveDate, String)>) -> PickupSchedule {
    let mut by_type: BTreeMap<String, Vec<NaiveDate>> = BTreeMap::new();
    for (date, waste_type) in pickups {
        by_type.entry(waste_type).or_default().push(date);
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
    PickupSchedule {
        address: address.to_string(),
        series,
    }
}

fn first_of_month_offset(today: NaiveDate, months_ahead: u32) -> NaiveDate {
    let total = today.year() * 12 + today.month0() as i32 + months_ahead as i32;
    NaiveDate::from_ymd_opt(total.div_euclid(12), total.rem_euclid(12) as u32 + 1, 1)
        .unwrap_or(today)
}

#[async_trait]
impl Provider for Hassleholm {
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
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for hit in &hits {
            let value = display_address(hit);
            if seen.insert(value.clone()) {
                out.push(Suggestion { value });
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        // Suggestions are "GATA 1, ORT"; search on the street part and
        // re-match on the full string to recover the upstream alias.
        let street = address
            .rsplit_once(',')
            .map(|(s, _)| s.trim())
            .unwrap_or(address);
        let wanted = address.to_lowercase();
        let hits = self.search(street).await?;
        let Some(hit) = hits
            .iter()
            .find(|h| display_address(h).to_lowercase() == wanted)
        else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };

        let today = Utc::now()
            .with_timezone(&chrono_tz::Europe::Stockholm)
            .date_naive();
        let mut pickups = Vec::new();
        for offset in 0..MONTHS {
            let month = first_of_month_offset(today, offset);
            let body = self.fetch_month(&hit.alias, month).await?;
            pickups.extend(parse_month(&body)?);
        }
        Ok(build_schedule(&display_address(hit), pickups))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_response() {
        let json = r#"{"message":"Successfully fetched 2 items","status":200,"data":[
            {"uuid":"7a98a3ff","alias":"hmab-laddarevaegen-1-haessleholm",
             "address":"LADDAREVÄGEN 1","city":"HÄSSLEHOLM","designation":"KNALLHATTEN 5"},
            {"uuid":"09b52800","alias":"hmab-vaestergatan-3-vinsloev",
             "address":"VÄSTERGATAN 3","city":"VINSLÖV","designation":null}
        ]}"#;
        let hits = parse_search(json).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].alias, "hmab-laddarevaegen-1-haessleholm");
        assert_eq!(display_address(&hits[0]), "LADDAREVÄGEN 1, HÄSSLEHOLM");
    }

    #[test]
    fn parses_month_skipping_filler_days_and_empty_services() {
        let json = r#"{"calendarMonth":{"days":[
            {"date":"2026-08-31","day":31,"currentMonth":false,"weekday":false},
            {"date":"2026-09-01","day":1,"currentMonth":true,"services":[{"name":"Kärl1","cssClassName":"karl1"}]},
            {"date":"2026-09-02","day":2,"currentMonth":true,"services":[{"name":"Kärl2","cssClassName":"karl2"}]},
            {"date":"2026-09-03","day":3,"currentMonth":true,"services":null}
        ]},"alias":"x","route":"/"}"#;
        let pickups = parse_month(json).unwrap();
        assert_eq!(pickups.len(), 2);
        assert_eq!(
            pickups[0],
            (NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(), "Kärl1".into())
        );
    }

    #[test]
    fn month_without_calendar_is_empty() {
        assert!(parse_month(r#"{"route":"/"}"#).unwrap().is_empty());
    }

    #[test]
    fn build_schedule_groups_by_type_with_sorted_explicit_dates() {
        let d = |m, day| NaiveDate::from_ymd_opt(2026, m, day).unwrap();
        let schedule = build_schedule(
            "LADDAREVÄGEN 1, HÄSSLEHOLM",
            vec![
                (d(9, 15), "Kärl1".into()),
                (d(9, 1), "Kärl1".into()),
                (d(9, 2), "Kärl2".into()),
                (d(9, 9), "Trädgårdsavfall".into()),
            ],
        );
        assert_eq!(schedule.series.len(), 3);
        let karl1 = schedule
            .series
            .iter()
            .find(|s| s.waste_type == "Kärl1")
            .unwrap();
        assert_eq!(karl1.interval_weeks, None);
        assert_eq!(karl1.anchor, vec![d(9, 1), d(9, 15)]);
    }

    #[test]
    fn month_offsets_cross_year_boundary() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        assert_eq!(
            first_of_month_offset(today, 0),
            NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()
        );
        assert_eq!(
            first_of_month_offset(today, 2),
            NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()
        );
    }
}
