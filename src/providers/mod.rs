use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Serialize;

pub mod alvesta;
pub mod arjeplog;
pub mod arvidsjaur;
pub mod avfallsappen;
pub mod lsr;
pub mod optigon;
pub mod rambo;
pub mod stromstad;
pub mod sysav;
pub mod vattenmiljoresurs;
pub mod vmab;
pub mod edp_future;
pub mod exde;
pub mod hassleholm;
pub mod indecta;
pub mod roslagsvatten;
pub mod sitevision_fetchplanner;
pub mod sodertorn;
pub mod stockholm;
pub mod sundsvall;
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
    /// Extra labels for the index page that all resolve to `id()`. Used by
    /// SRV Återvinning where Botkyrka/Haninge/Huddinge/Nynäshamn/Salem all
    /// share one Södertörns-sida. Default: no aliases, index uses `name()`.
    fn index_aliases(&self) -> &'static [&'static str] {
        &[]
    }
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
            // SÅM — Samverkan Återvinning Miljö. Delad Indecta-tenant
            // (client `sam`) för fem GGVV-kommuner + Hylte. Datasetet
            // har 5 pipe-separerade fält per rad (tredje och femte
            // fältet är kundinformation som ignoreras av parsern).
            Arc::new(indecta::Indecta::new(
                http.clone(),
                indecta::Config {
                    id: "gislaved",
                    name: "Gislaved",
                    placeholder: "t.ex. Storgatan 1",
                    note: "Sophämtningsdata från SÅM (Samverkan Återvinning Miljö).",
                    client: "sam",
                    cities: &[
                        "Gislaved", "Anderstorp", "Reftele", "Smålandsstenar",
                        "Broaryd", "Burseryd", "Skeppshult", "Öreryd", "Hestra",
                    ],
                },
            )),
            Arc::new(indecta::Indecta::new(
                http.clone(),
                indecta::Config {
                    id: "gnosjo",
                    name: "Gnosjö",
                    placeholder: "t.ex. Storgatan 1",
                    note: "Sophämtningsdata från SÅM (samdrift).",
                    client: "sam",
                    cities: &["Gnosjö", "Hillerstorp", "Nissafors", "Kulltorp"],
                },
            )),
            Arc::new(indecta::Indecta::new(
                http.clone(),
                indecta::Config {
                    id: "hylte",
                    name: "Hylte",
                    placeholder: "t.ex. Storgatan 1",
                    note: "Sophämtningsdata från SÅM (samdrift).",
                    client: "sam",
                    cities: &[
                        "Hyltebruk", "Kinnared", "Torup", "Landeryd",
                        "Rydöbruk", "Unnaryd", "Långaryd",
                    ],
                },
            )),
            Arc::new(indecta::Indecta::new(
                http.clone(),
                indecta::Config {
                    id: "vaggeryd",
                    name: "Vaggeryd",
                    placeholder: "t.ex. Storgatan 1",
                    note: "Sophämtningsdata från SÅM (samdrift).",
                    client: "sam",
                    cities: &["Vaggeryd", "Skillingaryd", "Klevshult", "Åker"],
                },
            )),
            Arc::new(indecta::Indecta::new(
                http.clone(),
                indecta::Config {
                    id: "varnamo",
                    name: "Värnamo",
                    placeholder: "t.ex. Storgatan 1",
                    note: "Sophämtningsdata från SÅM (samdrift).",
                    client: "sam",
                    cities: &[
                        "Värnamo", "Bor", "Bredaryd", "Forsheda", "Horda",
                        "Rydaholm", "Kärda", "Lanna",
                    ],
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
            edp(edp_future::Config {
                id: "ale", name: "Ale",
                placeholder: "t.ex. Ledetvägen 6", note: edp_note,
                api_url: "https://edp.ale.se/FutureWeb/SimpleWastePickup",
                cities: None,
            }),
            edp(edp_future::Config {
                id: "kungalv", name: "Kungälv",
                placeholder: "t.ex. Komministergatan 4", note: edp_note,
                api_url: "https://minasidor-va-avfall.kungalv.se/FutureWeb/SimpleWastePickup",
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
        edp_providers.push(edp(edp_future::Config {
            id: "arboga", name: "Arboga",
            placeholder: "t.ex. Centrumleden 6", note: edp_note,
            api_url: vafab,
            cities: Some(&["Arboga"]),
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

        // Samhällsbyggnad Bergslagen (SBB) — Ljusnarsberg, Lindesberg,
        // Nora, Hällefors. Delad EDP FutureWeb-instans.
        let sbb = "https://futureweb.sbbergslagen.se/FutureWeb/SimpleWastePickup";
        edp_providers.push(edp(edp_future::Config {
            id: "lindesberg", name: "Lindesberg",
            placeholder: "t.ex. Kristinavägen 1", note: edp_note,
            api_url: sbb,
            cities: Some(&[
                "Lindesberg", "Frövi", "Storå", "Ramsberg", "Fellingsbro",
                "Vedevåg", "Guldsmedshyttan", "Gusselby", "Löa",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "nora", name: "Nora",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: sbb,
            cities: Some(&["Nora", "Gyttorp", "Pershyttan", "Ås", "Striberg"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "hallefors", name: "Hällefors",
            placeholder: "t.ex. Sikforsvägen 1", note: edp_note,
            api_url: sbb,
            cities: Some(&["Hällefors", "Grythyttan", "Sikfors", "Bredsjö"]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "ljusnarsberg", name: "Ljusnarsberg",
            placeholder: "t.ex. Kyrkvägen 1", note: edp_note,
            api_url: sbb,
            cities: Some(&["Kopparberg", "Ställdalen", "Bångbro", "Ljusnarsberg"]),
        }));

        // Vivab / FutureWebFalken — Falkenberg (Varberg körs på en
        // separat login-gated instans och täcks inte).
        edp_providers.push(edp(edp_future::Config {
            id: "falkenberg", name: "Falkenberg",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: "https://minasidor.vivab.info/FutureWebFalken/SimpleWastePickup",
            cities: Some(&[
                "Falkenberg", "Ullared", "Vessigebro", "Slöinge", "Långås",
                "Ätran", "Fegen", "Vinberg", "Skällinge", "Sibbarp",
                "Källsjö", "Okome", "Vinberg", "Morup",
            ]),
        }));

        // June Avfall & Miljö / FutureWebJuneBasic — Jönköping, Habo, Mullsjö.
        // Delad EDP-instans; filter per kommun via cities allow-list. Notera
        // att datasetet innehåller enstaka poster märkta "NÄSSJÖ" (Sandhem-
        // varianter) som filtreras bort automatiskt.
        let june = "https://minasidor.juneavfall.se/FutureWebJuneBasic/SimpleWastePickup";
        edp_providers.push(edp(edp_future::Config {
            id: "jonkoping", name: "Jönköping",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: june,
            cities: Some(&[
                "Jönköping", "Huskvarna", "Norrahammar", "Bankeryd", "Taberg",
                "Tenhult", "Kaxholmen", "Skärstad", "Månsarp", "Öggestorp",
                "Örserum", "Bottnaryd", "Barnarp", "Ölmstad", "Visingsö",
                "Gränna", "Hakarp", "Lekeryd",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "habo", name: "Habo",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: june,
            cities: Some(&[
                "Habo", "Furusjö", "Fiskebäck", "Baskarp", "Kärnekulla",
                "Brandstorp",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "mullsjo", name: "Mullsjö",
            placeholder: "t.ex. Kyrkgatan 1", note: edp_note,
            api_url: june,
            cities: Some(&["Mullsjö", "Bjurbäck", "Nykyrka", "Sandhem"]),
        }));

        // WBAB — Ludvika och Smedjebacken (två separata FutureWeb-instanser
        // på samma bolag).
        edp_providers.push(edp(edp_future::Config {
            id: "ludvika", name: "Ludvika",
            placeholder: "t.ex. Storgatan 1", note: edp_note,
            api_url: "https://futureweb.wbab.se/EDPFutureweb/SimpleWastePickup",
            cities: Some(&[
                "Ludvika", "Grängesberg", "Sunnansjö", "Blötberget", "Nyhammar",
                "Grangärde", "Saxdalen", "Fredriksberg",
            ]),
        }));
        edp_providers.push(edp(edp_future::Config {
            id: "smedjebacken", name: "Smedjebacken",
            placeholder: "t.ex. Vasagatan 1", note: edp_note,
            api_url: "https://futureweb.wbab.se/EDPFuturewebSmedjebacken/SimpleWastePickup",
            cities: Some(&[
                "Smedjebacken", "Söderbärke", "Vad", "Hagge", "Morgårdshammar",
            ]),
        }));

        // Kramfors — enskild EDP-instans (FutureWebBasic).
        edp_providers.push(edp(edp_future::Config {
            id: "kramfors", name: "Kramfors",
            placeholder: "t.ex. Kungsgatan 1", note: edp_note,
            api_url: "https://futureweb.kramfors.se/EDPFutureWebBasic/SimpleWastePickup",
            cities: Some(&[
                "Kramfors", "Nyland", "Bollstabruk", "Docksta", "Ullånger",
                "Nordingrå", "Lugnvik", "Bjärtrå", "Frånö", "Salsåker",
                "Nyadal", "Norrfällsviken",
            ]),
        }));

        // Lidingö — enskild kommun, adress-etikett saknar city-suffix så
        // vi låter allowlisten vara tom (matchar allt).
        edp_providers.push(edp(edp_future::Config {
            id: "lidingo", name: "Lidingö",
            placeholder: "t.ex. Stockholmsvägen 1", note: edp_note,
            api_url: "https://vaochavfall.lidingo.se/Futureweb/SimpleWastePickup",
            cities: None,
        }));

        // Lund — LRV. Datasetet skriver "S Sandby" som förkortning för
        // Södra Sandby; båda formerna ingår för säkerhets skull.
        edp_providers.push(edp(edp_future::Config {
            id: "lund", name: "Lund",
            placeholder: "t.ex. Kyrkogatan 1", note: edp_note,
            api_url: "https://eservice431601.lund.se/lund/FutureWeb/SimpleWastePickup",
            cities: Some(&[
                "Lund", "S Sandby", "Södra Sandby", "Dalby", "Veberöd",
                "Genarp", "Torna Hällestad", "Revingeby", "Stångby",
            ]),
        }));

        // Svenljunga — enskild EDP-instans.
        edp_providers.push(edp(edp_future::Config {
            id: "svenljunga", name: "Svenljunga",
            placeholder: "t.ex. Kyrkogatan 1", note: edp_note,
            api_url: "https://edpfutureweb.svenljunga.se/FutureWeb/SimpleWastePickup",
            cities: Some(&[
                "Svenljunga", "Överlida", "Mjöbäck", "Sexdrega", "Kalv",
                "Håcksvik", "Mårdaklev", "Östra Frölunda", "Björketorp",
            ]),
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

        // Hässleholm Miljö — Appbolaget universal waste API for search,
        // SiteVision hsm.recycling-calendar webapp for the month data.
        let hassleholm_providers: Vec<Arc<dyn Provider>> =
            vec![Arc::new(hassleholm::Hassleholm::new(
                http.clone(),
                hassleholm::Config {
                    id: "hassleholm",
                    name: "Hässleholm",
                    placeholder: "t.ex. Vankivavägen 15",
                    note: "Sophämtningsdata från Hässleholm Miljö. Kalendern \
                           publiceras löpande — kommande månader dyker upp i \
                           flödet när de släpps.",
                    unit: "e34d7050-1b2a-4917-a921-0ea7742d0a6e",
                    calendar_url: "https://hassleholmmiljo.se/privat/sophamtning/tomningskalender",
                    portlet_id: "12.55ed8fe718ecb61f78a3204d",
                },
            ))];

        // Sundsvall — official CC0 open-data dataset published on
        // dataportal.se (61_6752). Single-kommun provider with an
        // in-memory cache refreshed every 12 h.
        let sundsvall_providers: Vec<Arc<dyn Provider>> =
            vec![Arc::new(sundsvall::Sundsvall::new(http.clone()))];

        // Arjeplog — statisk rutt-baserad kalender. Ingen extern
        // adressuppslag; provider genererar biweekly-datum lokalt från en
        // transkriberad rutt-tabell (arjeplog.se).
        let arjeplog_providers: Vec<Arc<dyn Provider>> =
            vec![Arc::new(arjeplog::Arjeplog::new())];

        // Arvidsjaur — samma mönster som Arjeplog. 13 slingor
        // transkriberade från arvidsjaur.se.
        let arvidsjaur_providers: Vec<Arc<dyn Provider>> =
            vec![Arc::new(arvidsjaur::Arvidsjaur::new())];

        // Rambo AB — Lysekil, Munkedal, Sotenäs, Tanum. Delad WP-JSON-
        // instans på rambo.se med statisk X-App-Identifier extraherad
        // ur pickup-widgetens JS-bundle.
        let rambo_p = |cfg: rambo::Config| -> Arc<dyn Provider> {
            Arc::new(rambo::Rambo::new(http.clone(), cfg))
        };
        let rambo_note = "Sophämtningsdata från Rambo AB. API:t returnerar \
                          bara nästa tömning per fraktion — kalendern uppdateras \
                          löpande när klienten hämtar in feeden på nytt.";
        let sysav_note = "Sophämtningsdata från Sysav via publik EDP-proxy. \
                          Endpointen returnerar bara nästa tömning per fraktion — \
                          kalendern uppdateras när klienten hämtar in feeden på nytt.";
        let sysav_p = |cfg: sysav::Config| -> Arc<dyn Provider> {
            Arc::new(sysav::Sysav::new(http.clone(), cfg))
        };
        let sysav_providers: Vec<Arc<dyn Provider>> = vec![
            sysav_p(sysav::Config {
                id: "kavlinge", name: "Kävlinge",
                placeholder: "t.ex. Storgatan 12", note: sysav_note,
                cities: &["Kävlinge", "Furulund", "Löddeköpinge", "Hofterup", "Barsebäck", "Barsebäckshamn"],
            }),
            sysav_p(sysav::Config {
                id: "lomma", name: "Lomma",
                placeholder: "t.ex. Storgatan 10", note: sysav_note,
                cities: &["Lomma", "Bjärred", "Borgeby", "Flädie"],
            }),
            sysav_p(sysav::Config {
                id: "svedala", name: "Svedala",
                placeholder: "t.ex. Storgatan 10", note: sysav_note,
                cities: &["Svedala", "Bara", "Klågerup", "Skabersjö", "Tjustorp"],
            }),
        ];

        // LSR — Landskrona-Svalövs Renhållnings AB. Publik REST-fasad
        // på minasidor.lsr.nu/api/api/external/ (POST JSON, ingen auth).
        let lsr_note = "Sophämtningsdata från LSR (Landskrona-Svalöv). \
                        API:t returnerar hela årsplanen som explicit \
                        datum-lista per fraktion.";
        let lsr_p = |cfg: lsr::Config| -> Arc<dyn Provider> {
            Arc::new(lsr::Lsr::new(http.clone(), cfg))
        };
        // Vatten och miljöresurs (VMR) — Härjedalen, Berg, Bräcke.
        // SiteVision-webbapp `garbage-collection` exponerar hela
        // adressdatasetet + frekvenskoder anonymt via portlet-route
        // /allAddresses. Vi genererar hämtningsdatum lokalt från
        // koderna (samma logik som widgetens JS).
        let vmr_note = "Sophämtningsdata från Vatten och miljöresurs. \
                        Frekvenskoderna avkodas lokalt (varje/varannan vecka × \
                        udda/jämna × veckodag) och datumen genereras framåt.";
        let vmr_p = |cfg: vattenmiljoresurs::Config| -> Arc<dyn Provider> {
            Arc::new(vattenmiljoresurs::VattenMiljoResurs::new(
                http.clone(),
                cfg,
            ))
        };
        let vmr_providers: Vec<Arc<dyn Provider>> = vec![
            vmr_p(vattenmiljoresurs::Config {
                id: "harjedalen", name: "Härjedalen",
                placeholder: "t.ex. Sonfjällsgatan 12", note: vmr_note,
                path: "harjedalen",
                portlet_id: "12.383d66bc198bb6c1bead0d",
            }),
            vmr_p(vattenmiljoresurs::Config {
                id: "berg", name: "Berg",
                placeholder: "t.ex. Storgatan 1", note: vmr_note,
                path: "berg",
                portlet_id: "12.383d66bc198bb6c1bead06",
            }),
            vmr_p(vattenmiljoresurs::Config {
                id: "bracke", name: "Bräcke",
                placeholder: "t.ex. Storgatan 1", note: vmr_note,
                path: "bracke",
                portlet_id: "12.383d66bc198bb6c1bead09",
            }),
        ];

        // Strömstad — SiteVision-widget som returnerar en HTML-tabell
        // för ?query=<adress>.
        let stromstad_providers: Vec<Arc<dyn Provider>> =
            vec![Arc::new(stromstad::Stromstad::new(http.clone()))];

        // VMAB / Rabadang — jQuery + fullcalendar-widget med två
        // anonyma PHP-endpoints. Samma software på cal-bromolla.vmab.se
        // och kalender.fyrfackronneby.se.
        let vmab_p = |cfg: vmab::Config| -> Arc<dyn Provider> {
            Arc::new(vmab::Vmab::new(http.clone(), cfg))
        };
        let vmab_providers: Vec<Arc<dyn Provider>> = vec![
            vmab_p(vmab::Config {
                id: "bromolla", name: "Bromölla",
                placeholder: "t.ex. Storgatan 1",
                note: "Sophämtningsdata från VMAB (Västblekinge Miljö AB). \
                       API:t returnerar 2+ års ordinarie hämtningsdagar per fraktion.",
                base_url: "https://cal-bromolla.vmab.se",
            }),
            vmab_p(vmab::Config {
                id: "ronneby", name: "Ronneby",
                placeholder: "t.ex. Kungsgatan 2",
                note: "Sophämtningsdata från Ronneby Miljöteknik (fyrfackskalendern). \
                       Samma software som VMAB — 2+ års ordinarie hämtningsdagar per fraktion.",
                base_url: "https://kalender.fyrfackronneby.se",
            }),
        ];

        // Optigon Avfallskollen — Forshaga, Grums, Hammarö. Publik
        // REST-fasad på avfallskollen-api.optigon.se (locations +
        // pickup-events per UUID).
        let optigon_note = "Sophämtningsdata via Optigon Avfallskollen. \
                            API:t returnerar hela årets hämtningar per fraktion.";
        let optigon_p = |cfg: optigon::Config| -> Arc<dyn Provider> {
            Arc::new(optigon::Optigon::new(http.clone(), cfg))
        };
        let optigon_providers: Vec<Arc<dyn Provider>> = vec![
            optigon_p(optigon::Config {
                id: "forshaga", name: "Forshaga",
                placeholder: "t.ex. Storgatan 1", note: optigon_note,
                cities: &["Forshaga", "Deje", "Olsäter"],
            }),
            optigon_p(optigon::Config {
                id: "grums", name: "Grums",
                placeholder: "t.ex. Storgatan 1", note: optigon_note,
                cities: &["Grums", "Slottsbron", "Slottbron", "Segmon", "Borgvik"],
            }),
            optigon_p(optigon::Config {
                id: "hammaro", name: "Hammarö",
                placeholder: "t.ex. Mörmovägen 1", note: optigon_note,
                cities: &["Hammarö"],
            }),
        ];

        let lsr_providers: Vec<Arc<dyn Provider>> = vec![
            lsr_p(lsr::Config {
                id: "landskrona", name: "Landskrona",
                placeholder: "t.ex. Storgatan 12", note: lsr_note,
                cities: &[
                    "Landskrona", "Häljarp", "Asmundtorp", "Glumslöv",
                    "Ålabodarna", "Ven", "Hilleshög", "Sankt Ibb",
                ],
            }),
            lsr_p(lsr::Config {
                id: "svalov", name: "Svalöv",
                placeholder: "t.ex. Storgatan 1", note: lsr_note,
                cities: &[
                    "Svalöv", "Teckomatorp", "Kågeröd", "Röstånga",
                    "Tågarp", "Billeberga",
                ],
            }),
        ];

        let rambo_providers: Vec<Arc<dyn Provider>> = vec![
            rambo_p(rambo::Config {
                id: "lysekil", name: "Lysekil",
                placeholder: "t.ex. Kungsgatan 1", note: rambo_note,
                cities: &["Lysekil", "Brastad", "Fiskebäckskil", "Grundsund", "Skaftö", "Rågårdsdal"],
            }),
            rambo_p(rambo::Config {
                id: "munkedal", name: "Munkedal",
                placeholder: "t.ex. Storgatan 1", note: rambo_note,
                cities: &["Munkedal", "Hedekas", "Håby", "Hällevadsholm", "Dingle"],
            }),
            rambo_p(rambo::Config {
                id: "sotenas", name: "Sotenäs",
                placeholder: "t.ex. Storgatan 1", note: rambo_note,
                cities: &[
                    "Kungshamn", "Hunnebostrand", "Bovallstrand", "Smögen",
                    "Väjern", "Malmön",
                ],
            }),
            rambo_p(rambo::Config {
                id: "tanum", name: "Tanum",
                placeholder: "t.ex. Storgatan 1", note: rambo_note,
                cities: &[
                    "Tanumshede", "Grebbestad", "Fjällbacka", "Kämpersvik",
                    "Rabbalshede", "Havstenssund", "Hamburgsund", "Lur", "Bullaren",
                ],
            }),
        ];

        // Alvesta — publikt bulk-JSON på arabschema.alvesta.se. SPA:n
        // hämtar hela datasetet (~2,5 MB) i förväg; vi speglar den
        // strategin med in-memory cache och 12 h TTL.
        let alvesta_providers: Vec<Arc<dyn Provider>> =
            vec![Arc::new(alvesta::Alvesta::new(http.clone()))];

        // SRV Återvinning — samlingsprovider "Södertörn" som täcker
        // Botkyrka, Haninge, Huddinge, Nynäshamn och Salem via ett
        // gemensamt öppet REST-API på srvatervinning.se.
        let sodertorn_providers: Vec<Arc<dyn Provider>> =
            vec![Arc::new(sodertorn::Sodertorn::new(http.clone()))];

        // Avfall & Återvinning Skaraborg (AÅS) — 13 kommuner täcks via
        // en delad Avfallsappen-widget (Bozzanova) på tenant
        // gullspang.avfallsapp.se. Statisk bearer + X-App-Identifier
        // extraherade ur widgetens Vue-bundle på avfallskaraborg.se.
        let aas = |cfg: avfallsappen::Config| -> Arc<dyn Provider> {
            Arc::new(avfallsappen::Avfallsappen::new(http.clone(), cfg))
        };
        let aas_tenant = "gullspang";
        let aas_bearer = "J6lD4hVH8pRMQZeBSoCvtCZj1V0wvgg0QvBqSDTH9fce942d";
        let aas_app_id = "70bae483-3268-4875-93f5-14f2274ec7cb";
        let aas_note = "Sophämtningsdata från Avfall & Återvinning Skaraborg (AÅS) \
                        via Avfallsappen. API:t returnerar bara nästa tömning \
                        per fraktion — kalendern uppdateras löpande när klienten \
                        hämtar in feeden på nytt.";
        let avfallsappen_providers: Vec<Arc<dyn Provider>> = vec![
            aas(avfallsappen::Config {
                id: "essunga", name: "Essunga",
                placeholder: "t.ex. Storgatan 1", note: aas_note,
                tenant: aas_tenant, bearer: aas_bearer, app_identifier: aas_app_id,
                cities: &["Nossebro", "Essunga"],
            }),
            aas(avfallsappen::Config {
                id: "falkoping", name: "Falköping",
                placeholder: "t.ex. Storgatan 10", note: aas_note,
                tenant: aas_tenant, bearer: aas_bearer, app_identifier: aas_app_id,
                cities: &[
                    "Falköping", "Floby", "Stenstorp", "Kättilstorp",
                    "Kinnarp", "Slutarp", "Åsarp", "Gudhem",
                    "Vartofta", "Broddetorp",
                ],
            }),
            aas(avfallsappen::Config {
                id: "grastorp", name: "Grästorp",
                placeholder: "t.ex. Storgatan 1", note: aas_note,
                tenant: aas_tenant, bearer: aas_bearer, app_identifier: aas_app_id,
                cities: &["Grästorp", "Tråvad"],
            }),
            aas(avfallsappen::Config {
                id: "gullspang", name: "Gullspång",
                placeholder: "t.ex. Storgatan 1", note: aas_note,
                tenant: aas_tenant, bearer: aas_bearer, app_identifier: aas_app_id,
                cities: &[
                    "Gullspång", "Hova", "Gårdsjö", "Otterbäcken",
                    "Skagersvik", "Aspa Bruk", "Aspabruk",
                ],
            }),
            aas(avfallsappen::Config {
                id: "gotene", name: "Götene",
                placeholder: "t.ex. Skolgatan 1", note: aas_note,
                tenant: aas_tenant, bearer: aas_bearer, app_identifier: aas_app_id,
                cities: &["Götene", "Källby", "Lundsbrunn", "Hällekis"],
            }),
            aas(avfallsappen::Config {
                id: "hjo", name: "Hjo",
                placeholder: "t.ex. Skolgatan 11", note: aas_note,
                tenant: aas_tenant, bearer: aas_bearer, app_identifier: aas_app_id,
                cities: &["Hjo", "Fagersanna"],
            }),
            aas(avfallsappen::Config {
                id: "karlsborg", name: "Karlsborg",
                placeholder: "t.ex. Storgatan 1", note: aas_note,
                tenant: aas_tenant, bearer: aas_bearer, app_identifier: aas_app_id,
                cities: &["Karlsborg", "Mölltorp", "Undenäs", "Forsvik"],
            }),
            aas(avfallsappen::Config {
                id: "mariestad", name: "Mariestad",
                placeholder: "t.ex. Kyrkogatan 1", note: aas_note,
                tenant: aas_tenant, bearer: aas_bearer, app_identifier: aas_app_id,
                cities: &["Mariestad", "Lyrestad", "Moholm", "Sjötorp", "Torsö"],
            }),
            aas(avfallsappen::Config {
                id: "skara", name: "Skara",
                placeholder: "t.ex. Skolgatan 1", note: aas_note,
                tenant: aas_tenant, bearer: aas_bearer, app_identifier: aas_app_id,
                cities: &["Skara", "Axvall", "Varnhem"],
            }),
            aas(avfallsappen::Config {
                id: "skovde", name: "Skövde",
                placeholder: "t.ex. Skolgatan 17", note: aas_note,
                tenant: aas_tenant, bearer: aas_bearer, app_identifier: aas_app_id,
                cities: &[
                    "Skövde", "Timmersdala", "Tidan", "Väring",
                    "Lerdala", "Värsås",
                ],
            }),
            aas(avfallsappen::Config {
                id: "tibro", name: "Tibro",
                placeholder: "t.ex. Skolgatan 10", note: aas_note,
                tenant: aas_tenant, bearer: aas_bearer, app_identifier: aas_app_id,
                cities: &["Tibro"],
            }),
            aas(avfallsappen::Config {
                id: "toreboda", name: "Töreboda",
                placeholder: "t.ex. Storgatan 1", note: aas_note,
                tenant: aas_tenant, bearer: aas_bearer, app_identifier: aas_app_id,
                cities: &["Töreboda", "Älgarås", "Finnerödja"],
            }),
            aas(avfallsappen::Config {
                id: "vara", name: "Vara",
                placeholder: "t.ex. Storgatan 1", note: aas_note,
                tenant: aas_tenant, bearer: aas_bearer, app_identifier: aas_app_id,
                cities: &["Vara", "Kvänum", "Vedum", "Stora Levene", "Larv"],
            }),
        ];

        Self {
            providers: providers
                .into_iter()
                .chain(edp_providers.into_iter())
                .chain(roslagsvatten_providers.into_iter())
                .chain(exde_providers.into_iter())
                .chain(hassleholm_providers.into_iter())
                .chain(sundsvall_providers.into_iter())
                .chain(sodertorn_providers.into_iter())
                .chain(avfallsappen_providers.into_iter())
                .chain(arjeplog_providers.into_iter())
                .chain(arvidsjaur_providers.into_iter())
                .chain(rambo_providers.into_iter())
                .chain(sysav_providers.into_iter())
                .chain(lsr_providers.into_iter())
                .chain(vmr_providers.into_iter())
                .chain(optigon_providers.into_iter())
                .chain(stromstad_providers.into_iter())
                .chain(vmab_providers.into_iter())
                .chain(alvesta_providers.into_iter())
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
