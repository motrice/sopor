//! Provider for SRV Återvinning — Södertörns kommunala avfallsbolag,
//! gemensamägt av Botkyrka, Haninge, Huddinge, Nynäshamn och Salem.
//!
//! SRV:s Mina Sidor är BankID-låst (Next.js-frontend mot BM FetchPlanner
//! på `fpservice.bmsystem.se/1839SRVProd/`), men den publika widgeten
//! på srvatervinning.se pratar med ett helt öppet REST-lager på samma
//! domän:
//!
//!   GET /rest-api/core/sewagePickup/getSuggestions?query=<term>
//!     -> { results: [{ name, customerId, address, city }] }
//!   GET /rest-api/core/sewagePickup/search?query=<street>&city=<CITY>
//!     -> { results: [{ customerId, zipCode, containers: [...] }],
//!          customerAddresses: [...], multipleAddresses: bool }
//!
//! Sökningen `query=` accepterar bara gatuadress; om autocomplete-strängen
//! `"Centralgatan 2, 14932 NYNÄSHAMN"` skickas rakt av returneras noll
//! träffar. Vi splittrar därför på första kommat och skickar postorten
//! som separat `city=`-parameter — det disambiguerar gator som finns i
//! flera SRV-orter (t.ex. Strandvägen i både Muskö och Dalarö).
//!
//! `calendars[]` innehåller redan förberäknade datum för hela året
//! (~52 events för veckotömning), så providern skickar dem rakt av som
//! explicita anchor-datum utan RRULE-projektion.

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::{Deserialize, Deserializer};

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const BASE: &str = "https://www.srvatervinning.se/rest-api/core/sewagePickup";

pub struct Sodertorn {
    http: reqwest::Client,
}

impl Sodertorn {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }
}

#[derive(Deserialize)]
struct SuggestionsResp {
    #[serde(default)]
    results: Vec<SuggestionItem>,
}

#[derive(Deserialize)]
struct SuggestionItem {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct SearchResp {
    #[serde(default)]
    results: Vec<SearchResult>,
}

#[derive(Deserialize, Default)]
struct SearchResult {
    #[serde(default)]
    containers: Vec<Container>,
}

#[derive(Deserialize, Default)]
struct Container {
    #[serde(default)]
    address: String,
    #[serde(default)]
    city: String,
    #[serde(default, rename = "zipCode")]
    zip_code: String,
    #[serde(default, rename = "containerType")]
    container_type: String,
    // SRV:s API är inkonsekvent här: strängen "Varje vecka" på veckotömda
    // kärl, strängen "False" på slam, och den blanka JSON-booleanen `false`
    // på FNI-utleveranser. Alla tre fångas som String; värdet slängs ändå
    // när hasFrequency=false.
    #[serde(default, deserialize_with = "string_or_bool")]
    frequency: String,
    #[serde(default, rename = "hasFrequency")]
    has_frequency: bool,
    #[serde(default)]
    calendars: Vec<Calendar>,
}

#[derive(Deserialize, Default)]
struct Calendar {
    #[serde(default, rename = "startDate")]
    start_date: String,
}

fn string_or_bool<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Either {
        S(String),
        B(serde::de::IgnoredAny),
    }
    match Either::deserialize(d)? {
        Either::S(s) => Ok(s),
        Either::B(_) => Ok(String::new()),
    }
}

#[async_trait]
impl Provider for Sodertorn {
    fn id(&self) -> &'static str {
        "sodertorn"
    }

    fn name(&self) -> &'static str {
        "Södertörn (SRV Återvinning)"
    }

    fn placeholder(&self) -> &'static str {
        "t.ex. Centralgatan 2"
    }

    fn note(&self) -> &'static str {
        "Sophämtningsdata från SRV Återvinning, det gemensamma \
         avfallsbolaget för Botkyrka, Haninge, Huddinge, Nynäshamn och \
         Salem. Skriv gatuadress — välj rätt ort ur autocomplete-listan \
         om gatan finns i flera orter."
    }

    fn index_aliases(&self) -> &'static [&'static str] {
        &["Botkyrka", "Haninge", "Huddinge", "Nynäshamn", "Salem"]
    }

    async fn autocomplete(&self, query: &str) -> Result<Vec<Suggestion>, ProviderError> {
        let q = query.trim();
        if q.len() < 2 {
            return Ok(vec![]);
        }
        let body = self
            .http
            .get(format!("{BASE}/getSuggestions"))
            .query(&[("query", q)])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        parse_suggestions(&body)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let (street, city) = split_address(address);
        let mut req = self.http.get(format!("{BASE}/search"));
        req = req.query(&[("query", street.as_str())]);
        if let Some(c) = city.as_deref() {
            req = req.query(&[("city", c)]);
        }
        let body = req.send().await?.error_for_status()?.text().await?;
        parse_schedule(address, &body)
    }
}

fn parse_suggestions(body: &str) -> Result<Vec<Suggestion>, ProviderError> {
    let resp: SuggestionsResp = serde_json::from_str(body)?;
    Ok(resp
        .results
        .into_iter()
        .filter(|s| !s.name.is_empty())
        .map(|s| Suggestion { value: s.name })
        .collect())
}

fn parse_schedule(query: &str, body: &str) -> Result<PickupSchedule, ProviderError> {
    let resp: SearchResp = serde_json::from_str(body)?;
    let Some(result) = resp.results.into_iter().next() else {
        return Ok(PickupSchedule {
            address: query.to_string(),
            series: vec![],
        });
    };

    let address = result
        .containers
        .first()
        .map(|c| {
            let mut parts = Vec::new();
            let street = c.address.trim();
            if !street.is_empty() {
                parts.push(street.to_string());
            }
            let tail = format!("{} {}", c.zip_code.trim(), c.city.trim());
            let tail = tail.trim();
            if !tail.is_empty() {
                parts.push(tail.to_string());
            }
            parts.join(", ")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| query.to_string());

    let series: Vec<PickupSeries> = result
        .containers
        .into_iter()
        .map(container_to_series)
        .filter(|s| !s.anchor.is_empty())
        .collect();

    Ok(PickupSchedule { address, series })
}

fn container_to_series(c: Container) -> PickupSeries {
    let mut dates: Vec<NaiveDate> = c
        .calendars
        .iter()
        .filter_map(|cal| NaiveDate::parse_from_str(&cal.start_date, "%Y-%m-%d").ok())
        .collect();
    dates.sort();
    dates.dedup();

    // "hasFrequency": false shows up on slam/spolning etc. where API:t
    // levererar en handfull konkreta datum utan cadence. Rensa bort den
    // falska "False"-strängen som annars visas i UI:t.
    let frequency_text = if c.has_frequency {
        c.frequency
    } else {
        String::new()
    };

    PickupSeries {
        waste_type: c.container_type,
        frequency_text,
        interval_weeks: None,
        anchor: dates,
    }
}

fn split_address(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    let Some((street, rest)) = trimmed.split_once(',') else {
        return (trimmed.to_string(), None);
    };
    let rest = rest.trim();
    // rest is either "<zip> <CITY>" or just "<CITY>". Split on first
    // whitespace; if the first token is a 5-digit postnummer, drop it.
    let city = match rest.split_once(char::is_whitespace) {
        Some((first, tail)) if first.chars().all(|c| c.is_ascii_digit()) => tail.trim(),
        _ => rest,
    };
    let city = city.trim();
    let city_opt = if city.is_empty() {
        None
    } else {
        Some(city.to_string())
    };
    (street.trim().to_string(), city_opt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_full_autocomplete_string() {
        let (street, city) = split_address("Centralgatan 2, 14932 NYNÄSHAMN");
        assert_eq!(street, "Centralgatan 2");
        assert_eq!(city.as_deref(), Some("NYNÄSHAMN"));
    }

    #[test]
    fn split_bare_street() {
        let (street, city) = split_address("Huddingevägen 303");
        assert_eq!(street, "Huddingevägen 303");
        assert!(city.is_none());
    }

    #[test]
    fn split_street_and_city_no_zip() {
        let (street, city) = split_address("Rönninge torg 2, RÖNNINGE");
        assert_eq!(street, "Rönninge torg 2");
        assert_eq!(city.as_deref(), Some("RÖNNINGE"));
    }

    #[test]
    fn split_multiword_city() {
        let (street, city) = split_address("Storgatan 1, 14830 Stora Vika");
        assert_eq!(street, "Storgatan 1");
        assert_eq!(city.as_deref(), Some("Stora Vika"));
    }

    #[test]
    fn suggestions_parse() {
        let body = r#"{"results":[
            {"name":"Vasavägen 1, 13770 DALARÖ","customerId":"366412","address":"Vasavägen 1","city":"DALARÖ"},
            {"name":"Vasavägen 1, 14132 HUDDINGE","customerId":"245363","address":"Vasavägen 1","city":"HUDDINGE"}
        ]}"#;
        let sugg = parse_suggestions(body).unwrap();
        assert_eq!(sugg.len(), 2);
        assert_eq!(sugg[0].value, "Vasavägen 1, 13770 DALARÖ");
        assert_eq!(sugg[1].value, "Vasavägen 1, 14132 HUDDINGE");
    }

    #[test]
    fn suggestions_empty_results_yields_empty() {
        let body = r#"{"results":[]}"#;
        assert!(parse_suggestions(body).unwrap().is_empty());
    }

    #[test]
    fn schedule_parses_weekly_container() {
        let body = r#"{
            "multipleAddresses": false,
            "customerAddresses": [],
            "results": [{
                "customerId": "913838",
                "zipCode": "14137",
                "containers": [{
                    "zipCode": "14137",
                    "address": "Huddingevägen 303",
                    "city": "HUDDINGE",
                    "containerType": "Kärl 240 liter grönt",
                    "firstCalendarDate": "2026-09-16",
                    "frequency": "Varje vecka",
                    "hasFrequency": true,
                    "calendars": [
                        {"startDate": "2026-09-16", "dayName": "Onsdag"},
                        {"startDate": "2026-09-23", "dayName": "Onsdag"},
                        {"startDate": "2026-09-30", "dayName": "Onsdag"}
                    ]
                }]
            }],
            "itemsPerPage": 10
        }"#;
        let sched = parse_schedule("Huddingevägen 303", body).unwrap();
        assert_eq!(sched.address, "Huddingevägen 303, 14137 HUDDINGE");
        assert_eq!(sched.series.len(), 1);
        let s = &sched.series[0];
        assert_eq!(s.waste_type, "Kärl 240 liter grönt");
        assert_eq!(s.frequency_text, "Varje vecka");
        assert_eq!(s.anchor.len(), 3);
        assert_eq!(
            s.anchor[0],
            NaiveDate::from_ymd_opt(2026, 9, 16).unwrap()
        );
        assert!(s.interval_weeks.is_none());
    }

    #[test]
    fn schedule_multiple_containers_become_separate_series() {
        let body = r#"{"results":[{
            "customerId":"343274","zipCode":"14932",
            "containers":[
                {"address":"Centralgatan 2","city":"NYNÄSHAMN","zipCode":"14932",
                 "containerType":"Kärl 140 liter mat","hasFrequency":true,
                 "frequency":"Varje vecka",
                 "calendars":[{"startDate":"2026-09-14"},{"startDate":"2026-09-21"}]},
                {"address":"Centralgatan 2","city":"NYNÄSHAMN","zipCode":"14932",
                 "containerType":"Kärl 660 liter grönt","hasFrequency":true,
                 "frequency":"Varje vecka",
                 "calendars":[{"startDate":"2026-09-11"},{"startDate":"2026-09-18"}]}
            ]
        }]}"#;
        let sched = parse_schedule("Centralgatan 2", body).unwrap();
        assert_eq!(sched.series.len(), 2);
        assert_eq!(sched.series[0].waste_type, "Kärl 140 liter mat");
        assert_eq!(sched.series[1].waste_type, "Kärl 660 liter grönt");
    }

    #[test]
    fn schedule_sludge_container_without_frequency() {
        let body = r#"{"results":[{
            "customerId":"20170","zipCode":"14831",
            "containers":[{
                "address":"Ösmo kyrka","city":"ÖSMO","zipCode":"14831",
                "containerType":"Slamavskiljare 1 kbm",
                "hasFrequency":false,"frequency":"False",
                "calendars":[
                    {"startDate":"2026-10-05"},
                    {"startDate":"2027-03-08"}
                ]
            }]
        }]}"#;
        let sched = parse_schedule("Ösmo kyrka", body).unwrap();
        assert_eq!(sched.series.len(), 1);
        let s = &sched.series[0];
        assert_eq!(s.waste_type, "Slamavskiljare 1 kbm");
        // "False"-strängen filtreras bort när hasFrequency=false.
        assert_eq!(s.frequency_text, "");
        assert_eq!(s.anchor.len(), 2);
    }

    #[test]
    fn schedule_empty_results_yields_empty_series() {
        let body = r#"{"multipleAddresses":false,"customerAddresses":[],"results":[],"itemsPerPage":10}"#;
        let sched = parse_schedule("Okänd väg 999", body).unwrap();
        assert_eq!(sched.address, "Okänd väg 999");
        assert!(sched.series.is_empty());
    }

    #[test]
    fn schedule_handles_boolean_frequency_field() {
        // Kärl 370 liter två fack i Vendelsö: frequency som JSON-bool `false`
        // istället för strängen "False". Måste parsa utan att krascha.
        let body = r#"{"results":[{
            "customerId":"334601","zipCode":"13672",
            "containers":[{
                "address":"Rönnvägen 3","city":"VENDELSÖ","zipCode":"13672",
                "containerType":"Kärl 370 liter två fack",
                "frequency":false,"hasFrequency":false,
                "firstCalendarDate":"2026-10-13",
                "calendars":[{"startDate":"2026-10-13","dayName":"Tisdag"}]
            }]
        }]}"#;
        let sched = parse_schedule("Rönnvägen 3", body).unwrap();
        assert_eq!(sched.series.len(), 1);
        assert_eq!(sched.series[0].waste_type, "Kärl 370 liter två fack");
        assert_eq!(sched.series[0].frequency_text, "");
        assert_eq!(sched.series[0].anchor.len(), 1);
    }

    #[test]
    fn schedule_drops_containers_with_no_dates() {
        let body = r#"{"results":[{"containers":[{
            "address":"Testv","city":"HUDDINGE","zipCode":"14100",
            "containerType":"Tömt","hasFrequency":true,"frequency":"Varje vecka",
            "calendars":[]
        }]}]}"#;
        let sched = parse_schedule("Testv", body).unwrap();
        assert!(sched.series.is_empty());
    }
}
