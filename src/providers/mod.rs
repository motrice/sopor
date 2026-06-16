use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Serialize;

pub mod sitevision_fetchplanner;
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

        use sitevision_fetchplanner::{Config, SitevisionFetchplanner};

        let providers: Vec<Arc<dyn Provider>> = vec![
            Arc::new(stockholm::Stockholm::new(http.clone())),
            Arc::new(SitevisionFetchplanner::new(
                http.clone(),
                Config {
                    id: "falun",
                    name: "Falun",
                    url: "https://fev.se/atervinning/sophamtning.html",
                    portlet_id: "12.1daee82819540d202c7322ce",
                    placeholder: "t.ex. Trotzgatan 13",
                    note: "Sophämtningsdata från Falu Energi & Vatten. \
                           Skriv enbart gatuadress (ingen kommun eller postnummer).",
                    default_city: "Falun",
                },
            )),
            Arc::new(SitevisionFetchplanner::new(
                http,
                Config {
                    id: "ornskoldsvik",
                    name: "Örnsköldsvik",
                    url: "https://miva.se/kundservice/sjalvservice/sophamtning/nar-kommer-sopbilen",
                    portlet_id: "12.5e486747177feaef88f29850",
                    placeholder: "t.ex. Storgatan 1",
                    note: "Sophämtningsdata från Miva (Örnsköldsviks kommun). \
                           Skriv enbart gatuadress (ingen kommun eller postnummer).",
                    default_city: "Örnsköldsvik",
                },
            )),
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
