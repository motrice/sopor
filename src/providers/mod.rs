use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Serialize;

pub mod edp_future;
pub mod exde;
pub mod indecta;
pub mod roslagsvatten;
pub mod sitevision_fetchplanner;
pub mod stockholm;
pub mod vasyd;

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
                http.clone(),
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
            Arc::new(vasyd::VaSyd::new(
                http.clone(),
                vasyd::Config {
                    id: "malmo",
                    name: "Malmö",
                    placeholder: "t.ex. Storgatan 1",
                    note: "Sophämtningsdata från VA SYD.",
                    cities: &[
                        "Malmö",
                        "Limhamn",
                        "Bunkeflostrand",
                        "Vintrie",
                        "Oxie",
                        "Tygelsjö",
                        "Klagshamn",
                    ],
                },
            )),
            Arc::new(vasyd::VaSyd::new(
                http.clone(),
                vasyd::Config {
                    id: "burlov",
                    name: "Burlöv",
                    placeholder: "t.ex. Storgatan 2",
                    note: "Sophämtningsdata från VA SYD.",
                    cities: &["Arlöv", "Åkarp", "Burlöv"],
                },
            )),
            Arc::new(indecta::Indecta::new(
                http.clone(),
                indecta::Config {
                    id: "ostra-goinge",
                    name: "Östra Göinge",
                    placeholder: "t.ex. Storgatan 1",
                    note: "Sophämtningsdata från Östra Göinge Renhållnings AB (OGRAB).",
                    client: "ograb",
                    cities: &[
                        "Broby",
                        "Glimåkra",
                        "Hanaskog",
                        "Hjärsås",
                        "Immeln",
                        "Knislinge",
                        "Kviinge",
                        "Sibbhult",
                    ],
                },
            )),
            Arc::new(indecta::Indecta::new(
                http.clone(),
                indecta::Config {
                    id: "osby",
                    name: "Osby",
                    placeholder: "t.ex. Storgatan 1",
                    note: "Sophämtningsdata från OGRAB (samdrift med Östra Göinge).",
                    client: "ograb",
                    cities: &["Osby", "Killeberg", "Lönsboda", "Visseltofta", "Hökön"],
                },
            )),
            Arc::new(indecta::Indecta::new(
                http.clone(),
                indecta::Config {
                    id: "sjobo",
                    name: "Sjöbo",
                    placeholder: "t.ex. Storgatan 1",
                    note: "Sophämtningsdata från Sjöbo kommun (Indecta-portal).",
                    client: "sjobo",
                    cities: &["Sjöbo", "Lövestad", "Vollsjö", "Blentarp"],
                },
            )),
        ];

        // EDP Future / SimpleWastePickup — shared JSON API across many
        // kommuner. Single-kommun deployments first, then multi-kommun
        // bolag split into per-kommun routes via `cities` allow-lists.
        let edp = |cfg: edp_future::Config| -> Arc<dyn Provider> {
            Arc::new(edp_future::EdpFuture::new(http.clone(), cfg))
        };

        let edp_note = "Sophämtningsdata via EDP Future / SimpleWastePickup. \
                        Skriv gatuadress utan postnummer.";

        let mut edp_providers: Vec<Arc<dyn Provider>> = vec![
            // Single-kommun deployments.
            edp(edp_future::Config {
                id: "skelleftea", name: "Skellefteå",
                placeholder: "t.ex. Frögatan 76", note: edp_note,
                api_url: "https://wwwtk2.skelleftea.se/FutureWeb/SimpleWastePickup",
                cities: None,
            }),
            edp(edp_future::Config {
                id: "boden", name: "Boden",
                placeholder: "t.ex. Kyrkgatan 24", note: edp_note,
                api_url: "https://edpmobile.boden.se/FutureWeb/SimpleWastePickup",
                cities: None,
            }),
            edp(edp_future::Config {
                id: "uppsala", name: "Uppsala",
                placeholder: "t.ex. Sadelvägen 1", note: edp_note,
                api_url: "https://futureweb.uppsalavatten.se/Uppsala/FutureWeb/SimpleWastePickup",
                cities: None,
            }),
            edp(edp_future::Config {
                id: "boras", name: "Borås",
                placeholder: "t.ex. Länghemsgatan 10", note: edp_note,
                api_url: "https://kundportal.borasem.se/EDPFutureWeb/SimpleWastePickup",
                cities: None,
            }),
            edp(edp_future::Config {
                id: "mark", name: "Mark",
                placeholder: "t.ex. Habyvägen 13", note: edp_note,
                api_url: "https://va-renhallning.mark.se/FutureWeb/SimpleWastePickup",
                cities: None,
            }),
            edp(edp_future::Config {
                id: "lycksele", name: "Lycksele",
                placeholder: "t.ex. Storgatan 1", note: edp_note,
                api_url: "https://future.lycksele.se/FutureWeb/SimpleWastePickup",
                cities: None,
            }),
            edp(edp_future::Config {
                id: "kiruna", name: "Kiruna",
                placeholder: "t.ex. Värmeverksvägen 12", note: edp_note,
                api_url: "https://kund.tekniskaverkenikiruna.se/FutureWebBasic/SimpleWastePickup",
                cities: None,
            }),
            edp(edp_future::Config {
                id: "lidkoping", name: "Lidköping",
                placeholder: "t.ex. Skaragatan 8", note: edp_note,
                api_url: "https://futureweb.lidkoping.se/FutureWebBasic/SimpleWastePickup",
                cities: None,
            }),
            edp(edp_future::Config {
                id: "stenungsund", name: "Stenungsund",
                placeholder: "t.ex. Strandvägen 15", note: edp_note,
                api_url: "https://futureweb.stenungsund.se/FutureWebBasic/SimpleWastePickup",
                cities: None,
            }),
            edp(edp_future::Config {
                id: "orust", name: "Orust",
                placeholder: "t.ex. Åvägen 2", note: edp_note,
                api_url: "https://va-renhallning-minasidor.orust.se/FutureWebBasic/SimpleWastePickup",
                cities: None,
            }),
            edp(edp_future::Config {
                id: "ljungby", name: "Ljungby",
                placeholder: "t.ex. Olofsgatan 9", note: edp_note,
                api_url: "https://edpwebb.ljungby.se/FutureWeb/SimpleWastePickup",
                cities: None,
            }),
            edp(edp_future::Config {
                id: "orebro", name: "Örebro",
                placeholder: "t.ex. Ringgatan 32", note: edp_note,
                api_url: "https://futureweb.orebro.se/FutureWeb/SimpleWastePickup",
                cities: None,
            }),
            edp(edp_future::Config {
                id: "nacka", name: "Nacka",
                placeholder: "t.ex. Fogdevägen 13", note: edp_note,
                api_url: "https://futureweb.nvoa.se/EDP/FutureWebBasic/SimpleWastePickup",
                cities: None,
            }),
        ];

        // Remondis-portalen täcker Herrljunga och Vårgårda.
        let remondis = "https://edpfuture.remondis.se/EDPFutureWeb/SimpleWastePickup";
        edp_providers.push(edp(edp_future::Config {
            id: "herrljunga", name: "Herrljunga",
            placeholder: "t.ex. Storgatan 5", note: edp_note,
            api_url: remondis,
            cities: Some(&["Herrljunga", "Annelund", "Ljung", "Hudene", "Mörlanda", "Eriksberg"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "vargarda", name: "Vårgårda",
            placeholder: "t.ex. Kungsgatan 21", note: edp_note,
            api_url: remondis,
            cities: Some(&["Vårgårda", "Tumberg", "Hol", "Asklanda", "Lena", "Fötene"]),
        }));

        // SSAM — Södra Smålands Avfall & Miljö. Täcker Lessebo, Markaryd,
        // Tingsryd, Älmhult, Växjö.
        let ssam = "https://edpfuture.ssam.se/FutureWeb/SimpleWastePickup";
        edp_providers.push(edp(edp_future::Config {
            id: "vaxjo", name: "Växjö",
            placeholder: "t.ex. Asteroidvägen 1", note: edp_note,
            api_url: ssam,
            cities: Some(&[
                "Växjö", "Ingelstad", "Lammhult", "Gemla", "Vederslöv", "Tävelsås",
                "Rottne", "Braås", "Åryd", "Furuby", "Ryssby", "Dädesjö", "Nöbbele",
                "Uråsa", "Värends Nöbbele",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "almhult", name: "Älmhult",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ssam,
            cities: Some(&[
                "Älmhult", "Diö", "Liatorp", "Häradsbäck", "Eneryda",
                "Pjätteryd", "Virestad", "Delary",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "tingsryd", name: "Tingsryd",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ssam,
            cities: Some(&[
                "Tingsryd", "Ryd", "Linneryd", "Konga", "Urshult", "Väckelsång",
                "Rävemåla",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "markaryd", name: "Markaryd",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ssam,
            cities: Some(&[
                "Markaryd", "Strömsnäsbruk", "Traryd", "Hinneryd", "Vivljunga",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "lessebo", name: "Lessebo",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ssam,
            cities: Some(&[
                "Lessebo", "Hovmantorp", "Kosta", "Ekeberga", "Skruv",
            ]),
        }));

        // Vafab Miljö — Västmanland, Enköping, Heby. Täcker ~11 kommuner.
        let vafab = "https://services.vafabmiljo.se/FutureWebVKFHus/SimpleWastePickup";
        edp_providers.push(edp(edp_future::Config {
            id: "vasteras", name: "Västerås",
            placeholder: "t.ex. Stora Gatan 1", note: edp_note,
            api_url: vafab,
            cities: Some(&[
                "Västerås", "Skerike", "Tortuna", "Tillberga", "Sevalla",
                "Romfartuna", "Kungsåra", "Skultuna", "Barkarö", "Dingtuna",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "enkoping", name: "Enköping",
            placeholder: "t.ex. Stora Gatan 1", note: edp_note,
            api_url: vafab,
            cities: Some(&[
                "Enköping", "Grillby", "Hummelsta", "Örsundsbro", "Lillkyrka",
                "Veckholm", "Tillinge", "Boglösa", "Fjärdhundra",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "hallstahammar", name: "Hallstahammar",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: vafab,
            cities: Some(&["Hallstahammar", "Kolbäck", "Strömsholm", "Sörstafors", "Berg"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "heby", name: "Heby",
            placeholder: "t.ex. Kyrkogatan 1", note: edp_note,
            api_url: vafab,
            cities: Some(&["Heby", "Tärnsjö", "Östervåla", "Morgongåva", "Vittinge", "Harbo"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "koping", name: "Köping",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: vafab,
            cities: Some(&["Köping", "Munktorp", "Kolsva", "Odensvi"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "norberg", name: "Norberg",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: vafab,
            cities: Some(&["Norberg", "Karbenning", "Kärrgruvan"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "sala", name: "Sala",
            placeholder: "t.ex. Stora Torget 1", note: edp_note,
            api_url: vafab,
            cities: Some(&["Sala", "Ranstad", "Möklinta", "Sätrabrunn", "Västerfärnebo"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "skinnskatteberg", name: "Skinnskatteberg",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: vafab,
            cities: Some(&["Skinnskatteberg", "Riddarhyttan"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "surahammar", name: "Surahammar",
            placeholder: "t.ex. Kyrkogatan 1", note: edp_note,
            api_url: vafab,
            cities: Some(&["Surahammar", "Virsbo", "Ramnäs"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "fagersta", name: "Fagersta",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: vafab,
            cities: Some(&["Fagersta", "Ängelsberg", "Västanfors"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "kungsor", name: "Kungsör",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: vafab,
            cities: Some(&["Kungsör", "Valskog", "Torpa"]),
        }));

        // Kretslopp Sydost — Kalmar län m.fl.
        let ksydost = "https://kundportal.kretsloppsydost.se/FutureWeb/SimpleWastePickup";
        edp_providers.push(edp(edp_future::Config {
            id: "kalmar", name: "Kalmar",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ksydost,
            cities: Some(&[
                "Kalmar", "Smedby", "Trekanten", "Påryd", "Ljungbyholm",
                "Lindsdal", "Rockneby", "Halltorp",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "morbylanga", name: "Mörbylånga",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ksydost,
            cities: Some(&[
                "Mörbylånga", "Färjestaden", "Glömminge", "Algutsrum",
                "Norra Möckleby", "Vickleby", "Degerhamn",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "nybro", name: "Nybro",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ksydost,
            cities: Some(&["Nybro", "Alsterbro", "Bäckebo", "Alsterfors", "Orrefors", "Madesjö"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "oskarshamn", name: "Oskarshamn",
            placeholder: "t.ex. Stångågatan 1", note: edp_note,
            api_url: ksydost,
            cities: Some(&["Oskarshamn", "Påskallavik", "Kristdala", "Misterhult", "Fårbo"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "torsas", name: "Torsås",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ksydost,
            cities: Some(&["Torsås", "Bergkvara", "Söderåkra", "Gullabo"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "borgholm", name: "Borgholm",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ksydost,
            cities: Some(&[
                "Borgholm", "Köpingsvik", "Löttorp", "Byxelkrok", "Runsten",
                "Föra", "Persnäs",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "monsteras", name: "Mönsterås",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ksydost,
            cities: Some(&["Mönsterås", "Timmernabben", "Blomstermåla", "Fliseryd"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "hultsfred", name: "Hultsfred",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ksydost,
            cities: Some(&[
                "Hultsfred", "Vena", "Virserum", "Målilla", "Mörlunda", "Silverdalen",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "hogsby", name: "Högsby",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ksydost,
            cities: Some(&["Högsby", "Berga", "Ruda", "Fågelfors", "Fagerhult"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "vetlanda", name: "Vetlanda",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ksydost,
            cities: Some(&["Vetlanda", "Korsberga", "Bäckaby", "Landsbro", "Myresjö", "Ekenässjön"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "savsjo", name: "Sävsjö",
            placeholder: "t.ex. Hägnevägen 1", note: edp_note,
            api_url: ksydost,
            cities: Some(&["Sävsjö", "Vrigstad", "Stockaryd", "Hultagård", "Rörvik"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "uppvidinge", name: "Uppvidinge",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: ksydost,
            cities: Some(&["Åseda", "Lenhovda", "Norrhult", "Älghult", "Alstermo", "Klavreström"]),
        }));

        // Roslagsvatten — Drupal-baserad widget för Ekerö, Vaxholm,
        // Österåker. Knivsta och Vallentuna har migrerats bort.
        let rv = |cfg: roslagsvatten::Config| -> Arc<dyn Provider> {
            Arc::new(roslagsvatten::Roslagsvatten::new(http.clone(), cfg))
        };
        let rv_note = "Sophämtningsdata från Roslagsvatten.";
        let roslagsvatten_providers: Vec<Arc<dyn Provider>> = vec![
            rv(roslagsvatten::Config {
                id: "ekero", name: "Ekerö",
                placeholder: "t.ex. Storgatan 1", note: rv_note,
                municipality: "ekero",
            }),
            rv(roslagsvatten::Config {
                id: "vaxholm", name: "Vaxholm",
                placeholder: "t.ex. Hamngatan 1", note: rv_note,
                municipality: "vaxholm",
            }),
            rv(roslagsvatten::Config {
                id: "osteraker", name: "Österåker",
                placeholder: "t.ex. Andromedavägen 1", note: rv_note,
                municipality: "osteraker",
            }),
        ];

        // EXDE Systems Mina sidor — Danderyd, Täby (Azure-hosted),
        // Simrishamn + Tomelilla (via Ökrab, shared backend).
        let exde = |cfg: exde::Config| -> Arc<dyn Provider> {
            Arc::new(exde::Exde::new(http.clone(), cfg))
        };
        let exde_note = "Sophämtningsdata via EXDE Systems Mina sidor.";
        let okrab = "https://minasidor.okrab.se/MinaSidor_API/api/external";
        let exde_providers: Vec<Arc<dyn Provider>> = vec![
            exde(exde::Config {
                id: "danderyd", name: "Danderyd",
                placeholder: "t.ex. Mörbyvägen 1", note: exde_note,
                api_url: "https://minasidor-danderyd-az.exdesystems.se/api/api/external",
                cities: None,
            }),
            exde(exde::Config {
                id: "taby", name: "Täby",
                placeholder: "t.ex. Marknadsvägen 1", note: exde_note,
                api_url: "https://minasidor-taby-az.exdesystems.se/api/api/external",
                cities: None,
            }),
            exde(exde::Config {
                id: "simrishamn", name: "Simrishamn",
                placeholder: "t.ex. Storgatan 1", note: exde_note,
                api_url: okrab,
                cities: Some(&[
                    "SIMRISHAMN", "KIVIK", "SKILLINGE", "GISLÖV", "HAMMENHÖG",
                    "S:T OLOF", "GÄRSNÄS", "Ö TOMMARP", "TOMMARP", "BRANTEVIK",
                    "BORRBY", "VITABY", "RÖRUM",
                ]),
            }),
            exde(exde::Config {
                id: "tomelilla", name: "Tomelilla",
                placeholder: "t.ex. Storgatan 1", note: exde_note,
                api_url: okrab,
                cities: Some(&[
                    "TOMELILLA", "TOMELILLLA", "SMEDSTORP", "BRÖSARP",
                    "ONSLUNDA", "LÖVESTAD", "RAMSÅSA", "TJUSTORP", "ANDRARUM",
                ]),
            }),
        ];

        Self {
            providers: providers
                .into_iter()
                .chain(edp_providers.into_iter())
                .chain(roslagsvatten_providers.into_iter())
                .chain(exde_providers.into_iter())
                .collect(),
        }
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
