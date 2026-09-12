//! Strömstad kommun — SiteVision-sida med inline garbage-form som
//! anropar samma URL med `?query=<adress>`. Servern renderar en
//! HTML-tabell med de tre kolumnerna (Hämtadress, Vad hämtas, Vecka/dag)
//! plus distrikt.
//!
//! Widgeten visar bara "ordinarie hämtningsdag" (närmaste träff per
//! adress + fraktion). Vi emitterar dessa explicita datum och förlitar
//! oss på klientens 12 h refresh.

use async_trait::async_trait;
use chrono::NaiveDate;
use regex::Regex;
use reqwest::Client;
use std::collections::BTreeMap;
use std::sync::OnceLock;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const PAGE_URL: &str = "https://www.stromstad.se/byggaboochmiljo/avfallochatervinning/sophamtning.4.47f506a2157fa40ebf68316c.html";
const AUTOCOMPLETE_LIMIT: usize = 25;

pub struct Stromstad {
    http: Client,
}

impl Stromstad {
    pub fn new(http: Client) -> Self {
        Self { http }
    }

    async fn fetch(&self, query: &str) -> Result<String, ProviderError> {
        self.http
            .get(PAGE_URL)
            .query(&[("query", query)])
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .text()
            .await
            .map_err(|e| ProviderError(e.to_string()))
    }
}

#[async_trait]
impl Provider for Stromstad {
    fn id(&self) -> &'static str {
        "stromstad"
    }
    fn name(&self) -> &'static str {
        "Strömstad"
    }
    fn placeholder(&self) -> &'static str {
        "t.ex. Karlsgatan 5"
    }
    fn note(&self) -> &'static str {
        "Sophämtningsdata från Strömstads kommun. Widgeten visar bara \
         ordinarie hämtningsdag — kalendern uppdateras när klienten \
         hämtar in feeden på nytt."
    }

    async fn autocomplete(&self, query: &str) -> Result<Vec<Suggestion>, ProviderError> {
        let q = query.trim();
        if q.len() < 2 {
            return Ok(vec![]);
        }
        let html = self.fetch(q).await?;
        let rows = parse_rows(&html);
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for r in rows {
            if !seen.insert(r.address.clone()) {
                continue;
            }
            out.push(Suggestion { value: r.address });
            if out.len() >= AUTOCOMPLETE_LIMIT {
                break;
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let html = self.fetch(address).await?;
        let rows = parse_rows(&html);
        Ok(build_schedule(address.to_string(), &rows))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    address: String,
    waste_type: String,
    date: NaiveDate,
}

fn parse_rows(html: &str) -> Vec<Row> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(
            r"(?s)<tr>\s*<td>([^<]+)</td>\s*<td>([^<]+)</td>\s*<td>(\d{4}-\d{2}-\d{2})</td>",
        )
        .unwrap()
    });
    re.captures_iter(html)
        .filter_map(|c| {
            let addr = c.get(1)?.as_str().trim().to_string();
            let waste = c.get(2)?.as_str().trim().to_string();
            let date = NaiveDate::parse_from_str(c.get(3)?.as_str(), "%Y-%m-%d").ok()?;
            Some(Row {
                address: addr,
                waste_type: waste,
                date,
            })
        })
        .collect()
}

fn build_schedule(address: String, rows: &[Row]) -> PickupSchedule {
    // Filter to exactly the requested address (case-insensitive).
    let target = address.trim().to_lowercase();
    let mut by_type: BTreeMap<String, Vec<NaiveDate>> = BTreeMap::new();
    for r in rows {
        if r.address.trim().to_lowercase() != target {
            continue;
        }
        let entry = by_type.entry(r.waste_type.clone()).or_default();
        if !entry.contains(&r.date) {
            entry.push(r.date);
        }
    }
    let series = by_type
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
    fn parse_rows_extracts_from_table() {
        let html = r#"<table class="garbage-table">
        <tr><th>Hämtadress</th><th>Vad hämtas</th><th>Vecka/dag</th><th>Distrikt</th></tr>
        <tr><td>Karlsgatan 5</td><td>Restavfall</td><td>2026-09-09</td><td>10</td></tr>
        <tr><td>Karlsgatan 5</td><td>Matavfall</td><td>2026-09-09</td><td>10</td></tr>
        <tr><td>Karlsgatan 50</td><td>Restavfall</td><td>2026-09-14</td><td>10</td></tr>
        </table>"#;
        let rows = parse_rows(html);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].address, "Karlsgatan 5");
        assert_eq!(rows[0].waste_type, "Restavfall");
        assert_eq!(rows[0].date, NaiveDate::from_ymd_opt(2026, 9, 9).unwrap());
    }

    #[test]
    fn build_schedule_filters_by_address_and_dedups() {
        let rows = vec![
            Row {
                address: "Karlsgatan 5".into(),
                waste_type: "Restavfall".into(),
                date: NaiveDate::from_ymd_opt(2026, 9, 9).unwrap(),
            },
            Row {
                address: "Karlsgatan 5".into(),
                waste_type: "Matavfall".into(),
                date: NaiveDate::from_ymd_opt(2026, 9, 9).unwrap(),
            },
            Row {
                address: "Karlsgatan 50".into(),
                waste_type: "Restavfall".into(),
                date: NaiveDate::from_ymd_opt(2026, 9, 14).unwrap(),
            },
        ];
        let s = build_schedule("Karlsgatan 5".into(), &rows);
        assert_eq!(s.series.len(), 2);
        assert!(s.series.iter().any(|p| p.waste_type == "Restavfall"));
        assert!(s.series.iter().any(|p| p.waste_type == "Matavfall"));
    }
}
