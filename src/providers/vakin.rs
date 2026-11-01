//! Vakin (Umeå, Vindeln) och UMEVA (Nordmaling) — CGI BFUS bakom en
//! helt öppen SiteVision-proxy.
//!
//! "När kommer sopbilen"-sidan på vakin.se kör SiteVision-webbappen
//! `se.soleil.garbageTruckFetcher` (portlet `12.6252338c1a0c6bd088ed033`).
//! Webbappens två routes nås via SiteVisions standalone-URL:er:
//!
//!   GET <sida>?sv.target=<portlet>&sv.<portlet>.route=/getAddresses
//!              &pickupAddress=<prefix>
//!     -> { data: [{ label, value: { zipCode, address, city } }] }
//!   GET <sida>?sv.target=<portlet>&sv.<portlet>.route=/getcontainerdata
//!              &addressParams=<json>&startDate=<iso>&endDate=<iso>
//!     -> [{ executionDate, flexServiceGroup, contentTypes: [...] }]
//!
//! `X-Requested-With: XMLHttpRequest` är OBLIGATORISK — annars svarar
//! SiteVision med hela HTML-sidan (samma fallgrop som hassleholm.rs).
//!
//! Serversidan proxar vidare till CGI BFUS på
//! `api-bfus.vakin.se/EP/EnvironmentalService/GetScheduledExecutionsSummary/
//! cgi/<gata>/<ort>/<postnr>/<från>/<till>`. Den vägen är nyckelskyddad
//! (401 direkt), men widget-routen kräver ingen inloggning — detta är
//! första BFUS-tenanten vi kommer åt utan BankID.
//!
//! `addressParams` är suggestion-objektets `value` rakt av. Backenden
//! klipper `address` vid första kommat och kräver postnummer utan
//! mellanslag; postnumret finns bara i autocomplete-svaret, så det bakas
//! in i suggestion-strängen ("Granvägen 1, Vindeln (92232)") på samma
//! sätt som edp_future round-trippar sina byggnadssträngar. Saknas det
//! (handskriven URL) slår `resolve` upp adressen på nytt.
//!
//! Upstream levererar färdigberäknade datum två år framåt — vi tar tolv
//! månader och emitterar explicita anchor-datum utan RRULE-projektion.
//! Adresser utan abonnemang svarar med en tom lista, och
//! fastighetsbeteckningar med kolon (`Sjöbotten 1:19`) ger 400 från BFUS
//! eftersom kolonet bryter path-segmentet — båda fallen är upstreams.

use async_trait::async_trait;
use chrono::{Months, NaiveDate, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::collections::BTreeMap;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const PAGE_URL: &str = "https://www.vakin.se/kundserviceochetjanster/sophamtning/narkommersopbilen.4.6252338c1a0c6bd088ed01f.html";
const PORTLET: &str = "12.6252338c1a0c6bd088ed033";
const ROUTE_PARAM: &str = "sv.12.6252338c1a0c6bd088ed033.route";
const HORIZON_MONTHS: u32 = 12;
const AUTOCOMPLETE_LIMIT: usize = 25;

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    pub cities: &'static [&'static str],
}

pub struct Vakin {
    http: Client,
    cfg: Config,
}

impl Vakin {
    pub fn new(http: Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    fn matches_city(&self, city: &str) -> bool {
        let needle = city.trim().to_lowercase();
        self.cfg.cities.iter().any(|c| c.to_lowercase() == needle)
    }

    fn webapp(&self, route: &str) -> reqwest::RequestBuilder {
        self.http
            .get(PAGE_URL)
            .header("X-Requested-With", "XMLHttpRequest")
            .query(&[
                ("sv.target", PORTLET),
                (ROUTE_PARAM, route),
                ("svAjaxReqParam", "ajax"),
            ])
    }

    async fn search(&self, query: &str) -> Result<Vec<AddressHit>, ProviderError> {
        let body = self
            .webapp("/getAddresses")
            .query(&[("pickupAddress", query)])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        parse_addresses(&body)
    }

    // Postnumret saknas när adressen inte kommer från autocomplete.
    // Sökningen är prefixbaserad på gatuadressen, så vi söker på allt
    // före kommat och matchar hela etiketten i svaret.
    async fn resolve(&self, label: &str) -> Result<Option<AddressHit>, ProviderError> {
        let street = label.split(',').next().unwrap_or(label).trim();
        if street.is_empty() {
            return Ok(None);
        }
        let wanted = normalize(label);
        Ok(self
            .search(street)
            .await?
            .into_iter()
            .filter(|h| self.matches_city(&h.value.city))
            .find(|h| normalize(&h.display()) == wanted))
    }
}

#[derive(Deserialize)]
struct UpstreamError {
    error: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AddressResp {
    Err(UpstreamError),
    Ok {
        #[serde(default)]
        data: Vec<AddressHit>,
    },
}

#[derive(Deserialize)]
struct AddressHit {
    #[serde(default)]
    label: String,
    #[serde(default)]
    value: AddressValue,
}

impl AddressHit {
    fn display(&self) -> String {
        if self.label.trim().is_empty() {
            self.value.address.trim().to_string()
        } else {
            self.label.trim().to_string()
        }
    }

    fn suggestion(&self) -> String {
        let display = self.display();
        let zip = self.value.zip_code.trim();
        if zip.is_empty() {
            display
        } else {
            format!("{display} ({zip})")
        }
    }
}

#[derive(Deserialize, Default)]
struct AddressValue {
    #[serde(default, rename = "zipCode")]
    zip_code: String,
    #[serde(default)]
    address: String,
    #[serde(default)]
    city: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ScheduleResp {
    Err(UpstreamError),
    Entries(Vec<Entry>),
}

#[derive(Deserialize)]
struct Entry {
    #[serde(default, rename = "executionDate")]
    execution_date: String,
    #[serde(default, rename = "contentTypes")]
    content_types: Vec<String>,
}

#[async_trait]
impl Provider for Vakin {
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
        Ok(self
            .search(q)
            .await?
            .into_iter()
            .filter(|h| self.matches_city(&h.value.city))
            .take(AUTOCOMPLETE_LIMIT)
            .map(|h| Suggestion {
                value: h.suggestion(),
            })
            .collect())
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let (label, zip) = split_zip(address);
        let (label, zip) = match zip {
            Some(zip) => (label, zip),
            None => match self.resolve(&label).await? {
                Some(hit) => (hit.display(), hit.value.zip_code.trim().to_string()),
                None => {
                    return Ok(PickupSchedule {
                        address: label,
                        series: vec![],
                    })
                }
            },
        };

        let city = city_of(&label);
        if city.is_empty() || !self.matches_city(&city) {
            return Ok(PickupSchedule {
                address: label,
                series: vec![],
            });
        }

        let params = serde_json::json!({
            "zipCode": zip,
            "address": label,
            "city": city,
        })
        .to_string();
        let today = Utc::now()
            .with_timezone(&chrono_tz::Europe::Stockholm)
            .date_naive();
        let end = today
            .checked_add_months(Months::new(HORIZON_MONTHS))
            .unwrap_or(today);

        let start_date = today.format("%Y-%m-%d").to_string();
        let end_date = end.format("%Y-%m-%d").to_string();
        let body = self
            .webapp("/getcontainerdata")
            .query(&[
                ("addressParams", params.as_str()),
                ("startDate", start_date.as_str()),
                ("endDate", end_date.as_str()),
            ])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        Ok(build_schedule(&label, parse_entries(&body)?))
    }
}

fn parse_addresses(body: &str) -> Result<Vec<AddressHit>, ProviderError> {
    match serde_json::from_str::<AddressResp>(body)? {
        AddressResp::Err(e) => Err(ProviderError(format!("upstream: {}", e.error))),
        AddressResp::Ok { data } => Ok(data),
    }
}

fn parse_entries(body: &str) -> Result<Vec<Entry>, ProviderError> {
    match serde_json::from_str::<ScheduleResp>(body)? {
        ScheduleResp::Err(e) => Err(ProviderError(format!("upstream: {}", e.error))),
        ScheduleResp::Entries(entries) => Ok(entries),
    }
}

// Varje rad är ett hämtningstillfälle med de fraktioner som tas den dagen.
// Vi grupperar på fraktionsmängden (sorterad, så att upstreams växlande
// ordning "Restavfall, Matavfall" / "Matavfall, Restavfall" inte blir två
// serier med olika UID).
fn build_schedule(address: &str, entries: Vec<Entry>) -> PickupSchedule {
    let mut grouped: BTreeMap<String, Vec<NaiveDate>> = BTreeMap::new();
    for entry in entries {
        let Ok(date) = NaiveDate::parse_from_str(entry.execution_date.trim(), "%Y-%m-%d") else {
            continue;
        };
        let mut types: Vec<String> = entry
            .content_types
            .iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        types.sort();
        types.dedup();
        let key = if types.is_empty() {
            "Hushållsavfall".to_string()
        } else {
            types.join(", ")
        };
        grouped.entry(key).or_default().push(date);
    }

    let series = grouped
        .into_iter()
        .map(|(waste_type, mut dates)| {
            dates.sort();
            dates.dedup();
            PickupSeries {
                waste_type,
                frequency_text: cadence_text(&dates),
                interval_weeks: None,
                anchor: dates,
            }
        })
        .filter(|s| !s.anchor.is_empty())
        .collect();

    PickupSchedule {
        address: address.to_string(),
        series,
    }
}

// Postnumret ligger sist i suggestion-strängen: "Granvägen 1, Vindeln (92232)".
fn split_zip(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    let Some(rest) = trimmed.strip_suffix(')') else {
        return (trimmed.to_string(), None);
    };
    let Some((head, zip)) = rest.rsplit_once('(') else {
        return (trimmed.to_string(), None);
    };
    let zip: String = zip.chars().filter(|c| !c.is_whitespace()).collect();
    if zip.len() == 5 && zip.chars().all(|c| c.is_ascii_digit()) {
        (head.trim().to_string(), Some(zip))
    } else {
        (trimmed.to_string(), None)
    }
}

fn city_of(label: &str) -> String {
    label
        .rsplit_once(',')
        .map(|(_, city)| city.trim().to_string())
        .unwrap_or_default()
}

fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

// Upstream skickar bara datum, ingen frekvenstext. Helgflyttade turer gör
// intervallet ojämnt — då hoppar vi över texten helt istället för att gissa.
fn cadence_text(dates: &[NaiveDate]) -> String {
    if dates.len() < 3 {
        return String::new();
    }
    let mut diffs = dates.windows(2).map(|w| (w[1] - w[0]).num_days());
    let first = diffs.next().unwrap_or(0);
    if !diffs.all(|d| d == first) {
        return String::new();
    }
    if first <= 0 || first % 7 != 0 {
        return String::new();
    }
    match first / 7 {
        1 => "Varje vecka".into(),
        2 => "Varannan vecka".into(),
        weeks => format!("Var {weeks}:e vecka"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(cities: &'static [&'static str]) -> Vakin {
        Vakin::new(
            Client::new(),
            Config {
                id: "test",
                name: "Test",
                placeholder: "",
                note: "",
                cities,
            },
        )
    }

    #[test]
    fn parses_and_filters_addresses() {
        let json = r#"{"data":[
            {"id":1.043901947E9,"label":"Granvägen 1 A, Umeå",
             "value":{"zipCode":"90738","address":"Granvägen 1 A, Umeå","city":"UMEÅ"}},
            {"id":1.0E9,"label":"Granvägen 1, Vindeln",
             "value":{"zipCode":"92232","address":"Granvägen 1, Vindeln","city":"VINDELN"}}
        ]}"#;
        let hits = parse_addresses(json).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].suggestion(), "Granvägen 1 A, Umeå (90738)");

        let p = provider(&["Vindeln", "Åmsele"]);
        let kept: Vec<String> = hits
            .iter()
            .filter(|h| p.matches_city(&h.value.city))
            .map(|h| h.suggestion())
            .collect();
        assert_eq!(kept, vec!["Granvägen 1, Vindeln (92232)"]);
    }

    #[test]
    fn city_match_is_unicode_aware() {
        let p = provider(&["Åmsele", "Vindeln"]);
        assert!(p.matches_city("ÅMSELE"));
        assert!(p.matches_city("vindeln"));
        assert!(!p.matches_city("UMEÅ"));
    }

    #[test]
    fn splits_trailing_zip() {
        assert_eq!(
            split_zip("Granvägen 1, Vindeln (92232)"),
            (
                "Granvägen 1, Vindeln".to_string(),
                Some("92232".to_string())
            )
        );
        assert_eq!(
            split_zip("Granvägen 1, Vindeln (922 32)"),
            (
                "Granvägen 1, Vindeln".to_string(),
                Some("92232".to_string())
            )
        );
        assert_eq!(
            split_zip("Granvägen 1, Vindeln"),
            ("Granvägen 1, Vindeln".to_string(), None)
        );
        // Husnummer i parentes ska inte misstolkas som postnummer.
        assert_eq!(
            split_zip("Backen 623 (A), Hörnsjö"),
            ("Backen 623 (A), Hörnsjö".to_string(), None)
        );
        assert_eq!(city_of("Granvägen 1, Vindeln"), "Vindeln");
        assert_eq!(city_of("Granvägen 1"), "");
    }

    #[test]
    fn groups_entries_by_fraction_set() {
        // Upstream växlar ordningen på contentTypes mellan rader — båda
        // raderna ska landa i samma serie.
        let json = r#"[
            {"executionDateFormated":"Fredag 13 november","executionDate":"2026-11-13",
             "flexServiceGroup":"Tunna småhus","id":"408065",
             "contentTypes":["Restavfall","Matavfall"],"dayUntil":12},
            {"executionDateFormated":"Fredag 11 december","executionDate":"2026-12-11",
             "flexServiceGroup":"Tunna småhus","id":"408065",
             "contentTypes":["Matavfall","Restavfall"],"dayUntil":40},
            {"executionDateFormated":"Fredag 8 januari","executionDate":"2027-01-08",
             "flexServiceGroup":"Tunna småhus","id":"408065",
             "contentTypes":["Restavfall","Matavfall"],"dayUntil":68},
            {"executionDateFormated":"Fredag 27 november","executionDate":"2026-11-27",
             "flexServiceGroup":"Tunna småhus","id":"408066",
             "contentTypes":["Matavfall","Plastförpackningar"],"dayUntil":26}
        ]"#;
        let s = build_schedule("Granvägen 1, Vindeln", parse_entries(json).unwrap());
        assert_eq!(s.series.len(), 2);
        assert_eq!(s.series[0].waste_type, "Matavfall, Plastförpackningar");
        assert_eq!(s.series[1].waste_type, "Matavfall, Restavfall");
        assert_eq!(s.series[1].anchor.len(), 3);
        assert_eq!(s.series[1].frequency_text, "Var 4:e vecka");
        assert!(s.series[1].interval_weeks.is_none());
        assert_eq!(s.series[0].frequency_text, "");
    }

    #[test]
    fn weekly_cadence_text() {
        let dates: Vec<NaiveDate> = ["2026-11-04", "2026-11-11", "2026-11-18"]
            .iter()
            .map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap())
            .collect();
        assert_eq!(cadence_text(&dates), "Varje vecka");
        // Helgflyttad tur -> ingen text istället för felaktig gissning.
        let shifted: Vec<NaiveDate> = ["2026-11-04", "2026-11-11", "2026-11-19"]
            .iter()
            .map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap())
            .collect();
        assert_eq!(cadence_text(&shifted), "");
        // Vindelns FNI-fraktioner går var sjätte vecka.
        let six: Vec<NaiveDate> = ["2026-11-27", "2027-01-08", "2027-02-19"]
            .iter()
            .map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap())
            .collect();
        assert_eq!(cadence_text(&six), "Var 6:e vecka");
    }

    #[test]
    fn addresses_without_subscription_give_no_series() {
        let s = build_schedule("Kvarnfors 118, Umeå", parse_entries("[]").unwrap());
        assert!(s.series.is_empty());
    }

    #[test]
    fn upstream_error_object_is_an_error() {
        let json = r#"{"error":"Fel vid hämtning: 400 - 400 Bad Request for URL https://api-bfus.vakin.se/EP/EnvironmentalService/GetScheduledExecutionsSummary/cgi/Sj%C3%B6botten%201%3A19/VINDELN/92292/2026-11-01/2027-11-01"}"#;
        assert!(parse_entries(json).is_err());
        assert!(parse_addresses(json).is_err());
    }
}
