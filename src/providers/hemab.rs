//! HEMAB (Härnösand Energi & Miljö AB) — sophämtningsdag per adress.
//!
//! Sidan `hemab.se/atervinning/soksophamtningsdag.4.…` är ett SiteVision-
//! sökportlet som normalt tar en gatuadress som fritext-query. Vi
//! utnyttjar att servern besvarar `?query=*` med *hela* datasetet på en
//! gång — 1790 adresser med veckodag + veckolista per fyrfackskärl.
//!
//!   GET https://www.hemab.se/4.<page>/12.<portlet>.htm
//!       ?state=ajaxQuery&isRenderingAjaxResult=true&query=<q>
//!
//! Datasetet cachas 12 h i minnet. Autocomplete gör substring-match
//! lokalt, schedule genererar datum lokalt genom att kombinera veckodag
//! med veckolistorna för Kärl 1 (mat/rest/tidningar/färgat glas) och
//! Kärl 2 (papper/plast/metall/ofärgat glas). Fritidshus-adresser har
//! bara veckor 18–40.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{Datelike, NaiveDate, Weekday};
use regex::Regex;
use tokio::sync::RwLock;
use tokio::time::Instant;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const BULK_URL: &str = "https://www.hemab.se/4.8575e6181a2a345d3ca8a6/\
                       12.8575e6181a2a345d3cb61f.htm\
                       ?state=ajaxQuery&isRenderingAjaxResult=true&query=%2A";
const TTL: Duration = Duration::from_secs(12 * 3600);
const AUTOCOMPLETE_LIMIT: usize = 25;

pub struct Hemab {
    http: reqwest::Client,
    cache: RwLock<Option<Arc<Dataset>>>,
}

struct Dataset {
    fetched_at: Instant,
    entries: Vec<Entry>,
}

#[derive(Debug, Clone)]
struct Entry {
    address: String,
    weekday: Weekday,
    kärl: Vec<Karl>,
}

#[derive(Debug, Clone)]
struct Karl {
    label: String,
    weeks: Vec<u32>,
    fritidshus: bool,
}

impl Hemab {
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
        let body = self
            .http
            .get(BULK_URL)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let entries = parse_hits(&body);
        let arc = Arc::new(Dataset {
            fetched_at: Instant::now(),
            entries,
        });
        *guard = Some(arc.clone());
        Ok(arc)
    }
}

#[async_trait]
impl Provider for Hemab {
    fn id(&self) -> &'static str {
        "harnosand"
    }
    fn name(&self) -> &'static str {
        "Härnösand"
    }
    fn placeholder(&self) -> &'static str {
        "t.ex. Nybrogatan eller Nyland 110"
    }
    fn note(&self) -> &'static str {
        "Sophämtningsdata från HEMAB. Skriv gatuadress *utan* husnummer \
         för centrala Härnösand (t.ex. \"Nybrogatan\"); för landsbygd/\
         fritidshus krävs adressnummer (t.ex. \"Nyland 110\"). Fyrfackskärl 1 \
         innehåller mat/rest/tidningar/färgat glas; Fyrfackskärl 2 innehåller \
         papper/plast/metall/ofärgat glas."
    }

    async fn autocomplete(&self, query: &str) -> Result<Vec<Suggestion>, ProviderError> {
        let q = query.trim().to_lowercase();
        if q.len() < 2 {
            return Ok(vec![]);
        }
        let ds = self.dataset().await?;
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for e in &ds.entries {
            if !e.address.to_lowercase().contains(&q) {
                continue;
            }
            if !seen.insert(e.address.clone()) {
                continue;
            }
            out.push(Suggestion {
                value: e.address.clone(),
            });
            if out.len() >= AUTOCOMPLETE_LIMIT {
                break;
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let ds = self.dataset().await?;
        let target = address.trim().to_lowercase();
        let matches: Vec<&Entry> = ds
            .entries
            .iter()
            .filter(|e| e.address.to_lowercase() == target)
            .collect();
        if matches.is_empty() {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        }
        let year = chrono::Local::now().year();
        Ok(build_schedule(address.to_string(), matches, year))
    }
}

fn parse_hits(html: &str) -> Vec<Entry> {
    let hit_re = Regex::new(r#"(?s)<li class="sv-search-hit[^"]*">(.*?)</li>"#).unwrap();
    let addr_re = Regex::new(r#"font-size: 15px;">([^<]+)</p>"#).unwrap();
    let day_re = Regex::new(r"<strong>Veckodag</strong>\s*([^<]+)</p>").unwrap();
    let bin_re = Regex::new(r#"class="(?:bin1|Bin2)">\s*<strong>Kärl</strong>\s*([^<]+)</p>"#).unwrap();
    let week_re = Regex::new(r#"class="Week[12]">\s*<strong>Veckor</strong>\s*([^<]+)</p>"#).unwrap();

    hit_re
        .captures_iter(html)
        .filter_map(|c| {
            let hit = c.get(1)?.as_str();
            let address = addr_re.captures(hit)?.get(1)?.as_str().trim().to_string();
            let weekday = parse_weekday(day_re.captures(hit)?.get(1)?.as_str().trim())?;
            let labels: Vec<String> = bin_re
                .captures_iter(hit)
                .filter_map(|c| Some(c.get(1)?.as_str().trim().to_string()))
                .collect();
            let week_texts: Vec<&str> = week_re
                .captures_iter(hit)
                .filter_map(|c| Some(c.get(1)?.as_str().trim()))
                .collect();
            if labels.is_empty() || week_texts.is_empty() {
                return None;
            }
            let kärl: Vec<Karl> = labels
                .into_iter()
                .zip(week_texts.into_iter())
                .filter_map(|(label, text)| parse_karl(label, text))
                .collect();
            if kärl.is_empty() {
                return None;
            }
            Some(Entry {
                address,
                weekday,
                kärl,
            })
        })
        .collect()
}

fn parse_weekday(s: &str) -> Option<Weekday> {
    match s.to_lowercase().as_str() {
        "måndag" => Some(Weekday::Mon),
        "tisdag" => Some(Weekday::Tue),
        "onsdag" => Some(Weekday::Wed),
        "torsdag" => Some(Weekday::Thu),
        "fredag" => Some(Weekday::Fri),
        "lördag" => Some(Weekday::Sat),
        "söndag" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Parse a "Veckor" text like `Udda veckor: 1, 3, 5, …` or
/// `Fritidshus Jämna veckor: 18, 20, 22, …`.
fn parse_karl(label: String, text: &str) -> Option<Karl> {
    let fritidshus = text.starts_with("Fritidshus");
    let colon = text.find(':')?;
    let numbers = &text[colon + 1..];
    let weeks: Vec<u32> = numbers
        .split(',')
        .filter_map(|s| s.trim().parse::<u32>().ok())
        .filter(|w| (1..=53).contains(w))
        .collect();
    if weeks.is_empty() {
        return None;
    }
    Some(Karl {
        label,
        weeks,
        fritidshus,
    })
}

fn build_schedule(display_address: String, matches: Vec<&Entry>, year: i32) -> PickupSchedule {
    let mut series: Vec<PickupSeries> = Vec::new();
    let mut seen_kärl: HashSet<(String, bool)> = HashSet::new();
    for e in matches {
        for k in &e.kärl {
            if !seen_kärl.insert((k.label.clone(), k.fritidshus)) {
                continue;
            }
            let anchor: Vec<NaiveDate> = k
                .weeks
                .iter()
                .filter_map(|w| NaiveDate::from_isoywd_opt(year, *w, e.weekday))
                .collect();
            if anchor.is_empty() {
                continue;
            }
            let waste_type = if k.fritidshus {
                format!("{} (fritidshus)", k.label)
            } else {
                k.label.clone()
            };
            let frequency = format!(
                "{} — {} veckor per år",
                weekday_sv(e.weekday),
                k.weeks.len(),
            );
            series.push(PickupSeries {
                waste_type,
                frequency_text: frequency,
                interval_weeks: None,
                anchor,
            });
        }
    }
    PickupSchedule {
        address: display_address,
        series,
    }
}

fn weekday_sv(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "Måndag",
        Weekday::Tue => "Tisdag",
        Weekday::Wed => "Onsdag",
        Weekday::Thu => "Torsdag",
        Weekday::Fri => "Fredag",
        Weekday::Sat => "Lördag",
        Weekday::Sun => "Söndag",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_HTML: &str = r#"<ol>
    <li class="sv-search-hit ui-corner-all">
        <p style="font-size: 15px;">Nybrogatan 19</p>
        <div class="sort-info">
            <p class="day"><strong>Veckodag</strong> Måndag</p>
            <p class="bin1"><strong>Kärl</strong> Fyrfackskärl 2</p>
        </div>
        <p class="Week1"><strong>Veckor</strong> Jämna veckor: 2, 4, 6, 8, 10</p>
        <p class="Bin2"><strong>Kärl</strong> Fyrfackskärl 1</p>
        <p class="Week2"><strong>Veckor</strong> Jämna veckor: 2, 6, 10</p>
    </li>
    <li class="sv-search-hit ui-corner-all">
        <p style="font-size: 15px;">Antjärn 101</p>
        <div class="sort-info">
            <p class="day"><strong>Veckodag</strong> Torsdag</p>
            <p class="bin1"><strong>Kärl</strong> Fyrfackskärl 2</p>
        </div>
        <p class="Week1"><strong>Veckor</strong> Fritidshus Jämna veckor: 18, 20, 22, 24</p>
        <p class="Bin2"><strong>Kärl</strong> Fyrfackskärl 1</p>
        <p class="Week2"><strong>Veckor</strong> Fritidshus Jämna veckor: 20, 24</p>
    </li>
    </ol>"#;

    #[test]
    fn parse_hits_extracts_all_fields() {
        let entries = parse_hits(SAMPLE_HTML);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].address, "Nybrogatan 19");
        assert_eq!(entries[0].weekday, Weekday::Mon);
        assert_eq!(entries[0].kärl.len(), 2);
        assert_eq!(entries[0].kärl[0].label, "Fyrfackskärl 2");
        assert_eq!(entries[0].kärl[0].weeks, vec![2, 4, 6, 8, 10]);
        assert!(!entries[0].kärl[0].fritidshus);
        assert_eq!(entries[0].kärl[1].label, "Fyrfackskärl 1");
        assert_eq!(entries[0].kärl[1].weeks, vec![2, 6, 10]);
    }

    #[test]
    fn parse_hits_flags_fritidshus() {
        let entries = parse_hits(SAMPLE_HTML);
        assert!(entries[1].kärl[0].fritidshus);
        assert_eq!(entries[1].address, "Antjärn 101");
        assert_eq!(entries[1].kärl[0].weeks, vec![18, 20, 22, 24]);
    }

    #[test]
    fn parse_karl_handles_fritidshus_prefix() {
        let k = parse_karl("X".into(), "Fritidshus Jämna veckor: 18, 20, 22").unwrap();
        assert!(k.fritidshus);
        assert_eq!(k.weeks, vec![18, 20, 22]);
    }

    #[test]
    fn parse_karl_handles_udda_veckor() {
        let k = parse_karl("X".into(), "Udda veckor: 1, 3, 5, 51, 53").unwrap();
        assert!(!k.fritidshus);
        assert_eq!(k.weeks, vec![1, 3, 5, 51, 53]);
    }

    #[test]
    fn parse_karl_ignores_out_of_range_weeks() {
        let k = parse_karl("X".into(), "Jämna veckor: 0, 2, 54, 100").unwrap();
        assert_eq!(k.weeks, vec![2]);
    }

    #[test]
    fn build_schedule_uses_iso_week_lookup() {
        let entries = parse_hits(SAMPLE_HTML);
        let matches: Vec<&Entry> = entries.iter().filter(|e| e.address == "Nybrogatan 19").collect();
        let s = build_schedule("Nybrogatan 19".into(), matches, 2026);
        assert_eq!(s.address, "Nybrogatan 19");
        assert_eq!(s.series.len(), 2);
        assert_eq!(s.series[0].waste_type, "Fyrfackskärl 2");
        // 2026 vecka 2 måndag = 2026-01-05
        assert_eq!(
            s.series[0].anchor[0],
            NaiveDate::from_ymd_opt(2026, 1, 5).unwrap()
        );
        assert_eq!(s.series[0].anchor.len(), 5);
    }

    #[test]
    fn build_schedule_dedups_multiple_matching_entries() {
        // Same address with same kärl labels should only produce one series pair
        let entries = parse_hits(SAMPLE_HTML);
        let matches: Vec<&Entry> = vec![&entries[0], &entries[0]];
        let s = build_schedule("Nybrogatan 19".into(), matches, 2026);
        assert_eq!(s.series.len(), 2); // Kärl 1 and Kärl 2, not 4
    }

    #[test]
    fn parse_weekday_swedish_forms() {
        assert_eq!(parse_weekday("Måndag"), Some(Weekday::Mon));
        assert_eq!(parse_weekday("torsdag"), Some(Weekday::Thu));
        assert_eq!(parse_weekday("Fredag"), Some(Weekday::Fri));
        assert!(parse_weekday("Monday").is_none());
    }
}
