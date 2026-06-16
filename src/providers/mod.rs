use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Serialize;

pub mod falun;
pub mod stockholm;

#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct PickupSeries {
    pub waste_type: String,
    pub frequency_text: String,
    pub interval_weeks: Option<u32>,
    pub anchor: Vec<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct PickupSchedule {
    pub address: String,
    pub series: Vec<PickupSeries>,
}

#[derive(Debug)]
pub struct ProviderError(pub String);

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ProviderError {}

impl From<reqwest::Error> for ProviderError {
    fn from(e: reqwest::Error) -> Self {
        ProviderError(format!("upstream: {e}"))
    }
}

impl From<serde_json::Error> for ProviderError {
    fn from(e: serde_json::Error) -> Self {
        ProviderError(format!("parse: {e}"))
    }
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn placeholder(&self) -> &'static str;
    fn note(&self) -> &'static str;
    async fn autocomplete(&self, query: &str) -> Result<Vec<Suggestion>, ProviderError>;
    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError>;
}

pub struct Registry {
    providers: Vec<Arc<dyn Provider>>,
}

impl Registry {
    pub fn build() -> Self {
        let http = reqwest::Client::builder()
            .user_agent("sopor/0.1 (+https://github.com/motrice/sopor) calendar bridge")
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .expect("reqwest client");

        let providers: Vec<Arc<dyn Provider>> = vec![
            Arc::new(stockholm::Stockholm::new(http.clone())),
            Arc::new(falun::Falun::new(http)),
        ];
        Self { providers }
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Provider>> {
        self.providers
            .iter()
            .find(|p| p.id() == id)
            .map(Arc::clone)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Provider>> {
        self.providers.iter()
    }
}
