//! Provider for kommuner using Indecta's "OnlineKalender" widget hosted at
//! `webbservice.indecta.se/kunder/<client>/kalender/`. One `Indecta` instance
//! is configured per kommun-route, but multiple kommuner can share the same
//! upstream `client` slug (e.g. OGRAB serves both Östra Göinge and Osby).
//! A `cities` allow-list per kommun filters the shared upstream's results.
//!
//! The platform is PHP 5.4-era and the address API is ISO-8859-1 over an
//! ad-hoc pipe-delimited text format; the schedule output is a 12-month HTML
//! grid. We parse it with regex rather than a full HTML parser to keep deps
//! minimal.

use std::collections::{BTreeMap, HashSet};

use async_trait::async_trait;
use chrono::NaiveDate;
use regex::Regex;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const BASE: &str = "https://webbservice.indecta.se/kunder";

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    /// Indecta client slug, e.g. `"ograb"`, `"sjobo"`.
    pub client: &'static str,
    /// Allow-list of locality names this kommun should display.
    pub cities: &'static [&'static str],
}

pub struct Indecta {
    http: reqwest::Client,
    cfg: Config,
}

impl Indecta {
    pub fn new(http: reqwest::Client, cfg: Config) -> Self {
        Self { http, cfg }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}/kalender/{}", BASE, self.cfg.client, path)
    }

    async fn fetch_text(&self, url: &str) -> Result<String, ProviderError> {
        let bytes = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        // Indecta deployments are inconsistent: OGRAB still serves Latin-1
        // for both endpoints, Sjöbo serves UTF-8 from the address API but
        // Latin-1 from the calendar HTML. Sniff per-response.
        Ok(decode_text(&bytes))
    }

    async fn fetch_addresses(&self, query: &str) -> Result<Vec<AddressRow>, ProviderError> {
        let url = format!(
            "{}?svar={}&limit=100",
            self.url("basfiler/laddaadresser.php"),
            latin1_url_encode(query.trim())
        );
        let body = self.fetch_text(&url).await?;
        Ok(parse_address_rows(&body))
    }

    async fn fetch_calendar(&self, row: &AddressRow) -> Result<String, ProviderError> {
        let mut url = format!(
            "{}?hsG={}&hsO={}",
            self.url("basfiler/onlinekalender.php"),
            latin1_url_encode(&row.street),
            latin1_url_encode(&row.city),
        );
        if !row.anlnr.is_empty() {
            url.push_str("&nrA=");
            url.push_str(&latin1_url_encode(&row.anlnr));
        }
        self.fetch_text(&url).await
    }

    fn locality_allowed(&self, city: &str) -> bool {
        self.cfg
            .cities
            .iter()
            .any(|c| c.eq_ignore_ascii_case(city))
    }
}

#[derive(Debug, Clone)]
struct AddressRow {
    street: String,
    city: String,
    #[allow(dead_code)]
    kundnr: String,
    anlnr: String,
}

fn parse_address_rows(body: &str) -> Vec<AddressRow> {
    body.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(4, '|').collect();
            if parts.len() < 2 {
                return None;
            }
            let street = parts[0].trim().to_string();
            let city = parts[1].trim().to_string();
            if street.is_empty() || city.is_empty() {
                return None;
            }
            let kundnr = parts.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
            let anlnr = parts.get(3).map(|s| s.trim().to_string()).unwrap_or_default();
            Some(AddressRow {
                street,
                city,
                kundnr,
                anlnr,
            })
        })
        .collect()
}

fn label_for(row: &AddressRow) -> String {
    format!("{}, {}", row.street, titlecase_city(&row.city))
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

fn decode_latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

fn decode_text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => decode_latin1(bytes),
    }
}

/// Percent-encode a string using ISO-8859-1 byte values. Any code point >= 256
/// is emitted as UTF-8 bytes (best-effort fallback for non-Latin-1 input).
fn latin1_url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let cp = c as u32;
        if c == ' ' {
            out.push('+');
        } else if c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~') {
            out.push(c);
        } else if cp < 0x100 {
            out.push_str(&format!("%{:02X}", cp as u8));
        } else {
            for b in c.to_string().as_bytes() {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

#[async_trait]
impl Provider for Indecta {
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
        let rows = self.fetch_addresses(q).await?;
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for row in rows {
            if !self.locality_allowed(&row.city) {
                continue;
            }
            let label = label_for(&row);
            if seen.insert(label.clone()) {
                out.push(Suggestion { value: label });
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let street_part = address.split(',').next().unwrap_or(address).trim();
        let rows = self.fetch_addresses(street_part).await?;
        let Some(row) = rows.into_iter().find(|r| {
            self.locality_allowed(&r.city) && label_for(r).eq_ignore_ascii_case(address)
        }) else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };

        let html = self.fetch_calendar(&row).await?;
        let by_fraction = parse_calendar(&html);
        let mut series = Vec::new();
        for (waste_type, mut dates) in by_fraction {
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

fn parse_calendar(html: &str) -> BTreeMap<String, Vec<NaiveDate>> {
    let month_re = Regex::new(
        r#"<td[^>]*class="styleMonthName"[^>]*>\s*([A-Za-z\u{00C0}-\u{024F}]+)\s*-\s*(\d{4})\s*</td>"#,
    )
    .unwrap();
    let pickup_re = Regex::new(
        r#"(?s)class="styleDayHit"[^>]*>\s*<div[^>]*>(\d{1,2})</div>\s*</td>.{0,400}?dagMedTomClass([A-Za-z0-9]+)"#,
    )
    .unwrap();

    let mut out: BTreeMap<String, Vec<NaiveDate>> = BTreeMap::new();
    let months: Vec<_> = month_re.captures_iter(html).collect();

    for (i, m) in months.iter().enumerate() {
        let month_name = m.get(1).unwrap().as_str();
        let Ok(year) = m.get(2).unwrap().as_str().parse::<i32>() else {
            continue;
        };
        let Some(month_num) = swedish_month_to_num(month_name) else {
            continue;
        };
        let start = m.get(0).unwrap().end();
        let end = months
            .get(i + 1)
            .map(|n| n.get(0).unwrap().start())
            .unwrap_or(html.len());
        let chunk = &html[start..end];
        for cap in pickup_re.captures_iter(chunk) {
            let Ok(day) = cap.get(1).unwrap().as_str().parse::<u32>() else {
                continue;
            };
            let code = cap.get(2).unwrap().as_str();
            let Some(date) = NaiveDate::from_ymd_opt(year, month_num, day) else {
                continue;
            };
            out.entry(map_fraction(code)).or_default().push(date);
        }
    }

    out
}

fn swedish_month_to_num(name: &str) -> Option<u32> {
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

fn map_fraction(code: &str) -> String {
    match code.to_uppercase().as_str() {
        "RE" => "Restavfall".into(),
        "MA" => "Matavfall".into(),
        "PA" | "PK" => "Pappersförpackningar".into(),
        "PL" => "Plastförpackningar".into(),
        "TI" => "Tidningar".into(),
        "WE" => "Wellpapp".into(),
        "ME" => "Metallförpackningar".into(),
        "GL" => "Glasförpackningar".into(),
        "GF" => "Grovavfall".into(),
        "FA" => "Farligt avfall".into(),
        "TR" => "Trädgårdsavfall".into(),
        "1" => "Fyrfackskärl 1".into(),
        "2" => "Fyrfackskärl 2".into(),
        other => format!("Fraktion {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin1_roundtrip_decode() {
        // "Storgatan 1|GLIMÅKRA|25368|BB096101" in ISO-8859-1
        let bytes: &[u8] = b"Storgatan 1|GLIM\xC5KRA|25368|BB096101";
        let decoded = decode_latin1(bytes);
        assert_eq!(decoded, "Storgatan 1|GLIMÅKRA|25368|BB096101");
    }

    #[test]
    fn latin1_url_encoding() {
        assert_eq!(latin1_url_encode("Storgatan 1"), "Storgatan+1");
        assert_eq!(latin1_url_encode("GLIMÅKRA"), "GLIM%C5KRA");
        assert_eq!(latin1_url_encode("Älg"), "%C4lg");
    }

    #[test]
    fn parses_ograb_style_rows_with_anlnr() {
        let body = "Storgatan 1|GLIMÅKRA|25368|BB096101\n\
                    Storgatan 2|BROBY|23648|BB096100\n";
        let rows = parse_address_rows(body);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].street, "Storgatan 1");
        assert_eq!(rows[0].city, "GLIMÅKRA");
        assert_eq!(rows[0].anlnr, "BB096101");
    }

    #[test]
    fn parses_sjobo_style_rows_without_anlnr() {
        let body = "Storgatan 1|Vollsjö\n\
                    Storgatan 10|Lövestad\n";
        let rows = parse_address_rows(body);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].city, "Vollsjö");
        assert_eq!(rows[0].anlnr, "");
    }

    #[test]
    fn skips_lines_without_street_or_city() {
        let body = "\n|EmptyStreet|x|y\nNoCity\n";
        assert!(parse_address_rows(body).is_empty());
    }

    #[test]
    fn decode_text_picks_utf8_then_latin1() {
        // Valid UTF-8 stays UTF-8
        let utf8: &[u8] = "Sjöbo".as_bytes();
        assert_eq!(decode_text(utf8), "Sjöbo");
        // Standalone 0xC5 is invalid UTF-8 → falls back to Latin-1 decode
        let latin1: &[u8] = b"GLIM\xC5KRA";
        assert_eq!(decode_text(latin1), "GLIMÅKRA");
    }

    #[test]
    fn fraction_mapping_known_and_unknown() {
        assert_eq!(map_fraction("RE"), "Restavfall");
        assert_eq!(map_fraction("MA"), "Matavfall");
        assert_eq!(map_fraction("PA"), "Pappersförpackningar");
        assert_eq!(map_fraction("PK"), "Pappersförpackningar");
        assert_eq!(map_fraction("1"), "Fyrfackskärl 1");
        assert_eq!(map_fraction("XX"), "Fraktion XX");
    }

    #[test]
    fn swedish_month_lookup() {
        assert_eq!(swedish_month_to_num("Januari"), Some(1));
        assert_eq!(swedish_month_to_num("juni"), Some(6));
        assert_eq!(swedish_month_to_num("December"), Some(12));
        assert_eq!(swedish_month_to_num("Nej"), None);
    }

    #[test]
    fn calendar_parser_picks_up_dates_per_fraction() {
        // Minimal HTML with two months. Each pickup day has a styleDayHit cell
        // on the day number followed by a dagMedTomClassXX marker.
        let html = r#"
            <table class="styleMonth"><tr><td class="styleMonthName">Januari - 2026</td></tr></table>
            <td colspan="1" class="styleDayHit"><div class="styleInteIdag">7</div></td>
            </tr><tr><td class="VRE"><span class="dagMedTomClassRE">R</span></td>
            <td colspan="1" class="styleDayHit"><div class="styleInteIdag">14</div></td>
            </tr><tr><td class="VMA"><span class="dagMedTomClassMA">M</span></td>
            <table class="styleMonth"><tr><td class="styleMonthName">Februari - 2026</td></tr></table>
            <td colspan="1" class="styleDayHit"><div class="styleInteIdag">4</div></td>
            </tr><tr><td class="VRE"><span class="dagMedTomClassRE">R</span></td>
        "#;
        let parsed = parse_calendar(html);
        let rest = parsed.get("Restavfall").expect("Restavfall");
        assert_eq!(
            *rest,
            vec![
                NaiveDate::from_ymd_opt(2026, 1, 7).unwrap(),
                NaiveDate::from_ymd_opt(2026, 2, 4).unwrap(),
            ]
        );
        let mat = parsed.get("Matavfall").expect("Matavfall");
        assert_eq!(*mat, vec![NaiveDate::from_ymd_opt(2026, 1, 14).unwrap()]);
    }
}
