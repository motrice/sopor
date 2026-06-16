use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

const BASE: &str = "https://www.stockholmvattenochavfall.se/villa-och-radhus/avfallstjanster/nar-kommer-sopbilen";

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
}

impl Client {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent("sopor/0.1 (+https://github.com/) — calendar subscription bridge")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .expect("reqwest client");
        Self { http }
    }

    pub async fn autocomplete(&self, query: &str) -> Result<Vec<Suggestion>, reqwest::Error> {
        let url = format!("{BASE}/AutoCompleteMe");
        self.http
            .get(url)
            .query(&[("query", query)])
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<Suggestion>>()
            .await
    }

    pub async fn search(&self, address: &str) -> Result<Schedule, reqwest::Error> {
        let url = format!("{BASE}/Search");
        self.http
            .get(url)
            .query(&[("address", address)])
            .send()
            .await?
            .error_for_status()?
            .json::<Schedule>()
            .await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub value: String,
    pub data: String,
}

pub type Schedule = BTreeMap<String, Vec<Entry>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Entry {
    pub fetch_frequency: String,
    pub execution_date: String,
    pub weekday: String,
}
