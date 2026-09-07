use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Deserialize;

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

const BASE: &str =
    "https://www.stockholmvattenochavfall.se/villa-och-radhus/avfallstjanster/nar-kommer-sopbilen";

pub struct Stockholm {
    http: reqwest::Client,
}

impl Stockholm {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }
}

#[derive(Deserialize)]
struct SvoaSuggestion {
    value: String,
    #[serde(rename = "data")]
    _data: Option<String>,
}

// SVOA's Search endpoint returns a flat array of entries, one per pickup.
// Historical shape (PascalCase, keyed by waste type) was replaced in
// September 2026 by a camelCase array with an explicit `group` field.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SvoaEntry {
    group: String,
    fetch_frequency: String,
    execution_date: String,
    #[serde(default, rename = "weekday")]
    _weekday: String,
}

#[async_trait]
impl Provider for Stockholm {
    fn id(&self) -> &'static str {
        "stockholm"
    }
    fn name(&self) -> &'static str {
        "Stockholm"
    }
    fn placeholder(&self) -> &'static str {
        "t.ex. Olovslundsvägen 9"
    }
    fn note(&self) -> &'static str {
        "Endast villor och radhus i Stockholms stad (Stockholm Vatten och Avfall). \
         Flerfamiljshus och samfälligheter ger tomma resultat."
    }

    async fn autocomplete(&self, query: &str) -> Result<Vec<Suggestion>, ProviderError> {
        if query.trim().len() < 2 {
            return Ok(vec![]);
        }
        let url = format!("{BASE}/AutoCompleteMe");
        let raw: Vec<SvoaSuggestion> = self
            .http
            .get(url)
            .query(&[("query", query)])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(raw
            .into_iter()
            .map(|s| Suggestion { value: s.value })
            .collect())
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let url = format!("{BASE}/Search");
        let json = self
            .http
            .get(url)
            .query(&[("address", address)])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        parse_schedule(&json, address)
    }
}

fn parse_schedule(json: &str, address: &str) -> Result<PickupSchedule, ProviderError> {
    let entries: Vec<SvoaEntry> = serde_json::from_str(json)?;
    let mut series = Vec::new();
    for entry in entries {
        let Some(date) = NaiveDate::parse_from_str(&entry.execution_date, "%Y-%m-%d").ok() else {
            continue;
        };
        if entry.group.trim().is_empty() {
            continue;
        }
        series.push(PickupSeries {
            waste_type: entry.group,
            frequency_text: entry.fetch_frequency.clone(),
            interval_weeks: parse_interval_weeks(&entry.fetch_frequency),
            anchor: vec![date],
        });
    }
    Ok(PickupSchedule {
        address: address.to_string(),
        series,
    })
}

fn parse_interval_weeks(freq: &str) -> Option<u32> {
    let lower = freq.to_lowercase();
    if lower.contains("varje vecka") {
        return Some(1);
    }
    if lower.contains("varannan vecka") {
        return Some(2);
    }
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

    #[test]
    fn parses_known_frequencies() {
        assert_eq!(parse_interval_weeks("Varje vecka"), Some(1));
        assert_eq!(parse_interval_weeks("Varannan vecka"), Some(2));
        assert_eq!(parse_interval_weeks("Var 4:e vecka"), Some(4));
        assert_eq!(parse_interval_weeks("Var 8:e vecka"), Some(8));
        assert_eq!(parse_interval_weeks("Okänt format"), None);
    }

    #[test]
    fn parses_villa_schedule() {
        let json = r#"[
            {"group":"Matavfall, villa","fetchFrequency":"Varannan vecka","executionDate":"2026-06-30","weekday":"Tisdag"},
            {"group":"Restavfall, villa","fetchFrequency":"Var 4:e vecka","executionDate":"2026-06-30","weekday":"Tisdag"}
        ]"#;
        let schedule = parse_schedule(json, "Olovslundsvägen 9, Bromma, 167 72").unwrap();
        assert_eq!(schedule.address, "Olovslundsvägen 9, Bromma, 167 72");
        assert_eq!(schedule.series.len(), 2);
        let mat = schedule
            .series
            .iter()
            .find(|s| s.waste_type == "Matavfall, villa")
            .expect("matavfall");
        assert_eq!(mat.interval_weeks, Some(2));
        assert_eq!(mat.anchor, vec![NaiveDate::from_ymd_opt(2026, 6, 30).unwrap()]);
    }

    #[test]
    fn empty_response_yields_no_series() {
        let schedule = parse_schedule("[]", "Hornsgatan 1, Stockholm").unwrap();
        assert!(schedule.series.is_empty());
    }

    #[test]
    fn parses_live_svoa_response_shape() {
        // Captured verbatim from SVOA's Search endpoint 2026-09-06 while
        // debugging the "invalid type: sequence, expected a map" incident.
        let json = r#"[{"group":"Kärl 2 - papper och plast","fetchFrequency":"Varannan vecka","executionDate":"2026-09-09","weekday":"Onsdag"},{"group":"Matavfall, villa","fetchFrequency":"Varannan vecka","executionDate":"2026-09-09","weekday":"Onsdag"},{"group":"Kärl 3 - glas och metall","fetchFrequency":"Var 4:e vecka","executionDate":"2026-09-10","weekday":"Torsdag"},{"group":"Restavfall, villa","fetchFrequency":"Var 8:e vecka","executionDate":"2026-09-23","weekday":"Onsdag"}]"#;
        let schedule = parse_schedule(json, "Skördevägen 80, Enskededalen, 121 33").unwrap();
        assert_eq!(schedule.series.len(), 4);
        let rest = schedule
            .series
            .iter()
            .find(|s| s.waste_type == "Restavfall, villa")
            .expect("restavfall");
        assert_eq!(rest.interval_weeks, Some(8));
        assert_eq!(
            rest.anchor,
            vec![NaiveDate::from_ymd_opt(2026, 9, 23).unwrap()]
        );
    }

    #[test]
    fn skips_entries_with_blank_group() {
        let json = r#"[
            {"group":"","fetchFrequency":"Varje vecka","executionDate":"2026-06-30","weekday":"Tisdag"},
            {"group":"Restavfall, villa","fetchFrequency":"Var 8:e vecka","executionDate":"2026-09-23","weekday":"Onsdag"}
        ]"#;
        let schedule = parse_schedule(json, "X").unwrap();
        assert_eq!(schedule.series.len(), 1);
        assert_eq!(schedule.series[0].waste_type, "Restavfall, villa");
        assert_eq!(schedule.series[0].interval_weeks, Some(8));
    }
}
