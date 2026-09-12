//! Arvidsjaur — statisk ruttbaserad kalender, samma mönster som Arjeplog.
//! Kommunen publicerar 13 slingor (Slinga 1–13) med fast veckodag och
//! jämn/udda ISO-vecka. Ingen adress-uppslag; rutt-tabellen är
//! transkriberad från arvidsjaur.se och datumen genereras lokalt.
//!
//! Arvidsjaurs tätort är uppdelad i "område 1–4" som endast är markerade
//! på en karta på webbsidan. Vi kan inte översätta gatuadress → område;
//! användare i tätorten måste själva slå upp sitt område på kartan och
//! sedan välja motsvarande slinga.
//!
//! Överlapp med Arjeplog: flera slingor listar även byar i Arjeplogs
//! kommun (Slinga 2, 3, 6, 10, 11, 12). Källpagen för Arvidsjaur och
//! Arjeplog är inte alltid samstämmiga om vecko-pariteten (t.ex.
//! Slinga 3: arvidsjaur.se anger tisdag udda vecka, arjeplog.se anger
//! tisdag jämn vecka). Vi speglar respektive kommuns egen sida per
//! kommun-route.
//!
//! Källa (verifierad 2026-09-12):
//! https://arvidsjaur.se/byggabomiljo/avfallochatervinning/sophamtning.711.html

use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate, Weekday};

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Parity {
    Odd,
    Even,
}

struct Route {
    slinga: &'static str,
    weekday: Weekday,
    parity: Parity,
    areas: &'static [&'static str],
}

const ROUTES: &[Route] = &[
    Route {
        slinga: "1",
        weekday: Weekday::Mon,
        parity: Parity::Odd,
        areas: &[
            "Fjällbonäs", "Deppis", "Renträsk", "Pjesker", "Norra Holmnäs",
            "Voutner", "Lauker", "Åkroken", "Njallejaur", "Viståsen",
            "Rättsel", "Ljusträsk", "Pilträsk", "Östra granberg", "Lomträsk",
            "Auktsjaur", "Rönnberg", "Akkavare",
        ],
    },
    Route {
        slinga: "2",
        weekday: Weekday::Tue,
        parity: Parity::Odd,
        areas: &[
            "Björkliden", "Glottje", "Kronogård", "Nuortejaur", "Utterbacken",
            "Strittjomvare", "Östansjö", "Strömnäs", "Suddesjaure", "Brännudden",
            "Norrstrand", "Lappvallen", "Lövudden", "Abraure", "Gitton",
            "Varasviken", "Rävudden", "Norra Bergnäs", "Rebnis", "Örnvik",
            "Siksele", "Galtisjaure", "Stensund", "Svannäs", "Arjeplog",
        ],
    },
    Route {
        slinga: "3",
        weekday: Weekday::Tue,
        parity: Parity::Odd,
        areas: &[
            "Arjeplog", "Dainak", "Geigo", "Vaxborg", "Maskaure",
            "Gransel", "Innervik", "Racksund",
        ],
    },
    Route {
        slinga: "4",
        weekday: Weekday::Wed,
        parity: Parity::Odd,
        areas: &["Arvidsjaur område 1"],
    },
    Route {
        slinga: "5",
        weekday: Weekday::Thu,
        parity: Parity::Odd,
        areas: &[
            "Rönnliden", "Gardejaur", "Rismyrheden", "Hedberg",
            "Sjnjerravägen (Treskifte)", "Mausjaur", "Storberg", "Sjöträsk",
            "Renvallen", "Arvidsjaur",
        ],
    },
    Route {
        slinga: "6",
        weekday: Weekday::Thu,
        parity: Parity::Odd,
        areas: &["Arjeplog Öberget område 2", "Arvidsjaur område 4"],
    },
    Route {
        slinga: "7",
        weekday: Weekday::Fri,
        parity: Parity::Odd,
        areas: &[
            "Arvidsjaur", "Abborrträsk", "Saltmyran", "Hålbergsliden",
            "Svartliden", "Snöbränna", "Brännberg", "Lappträsk/Moräng",
            "Svanträsk", "Järvträsk", "Högheden", "Södra Sandträsk",
            "Utterliden", "Baktsjaur", "Bäcknäs", "Grundträsk", "Ånäset",
            "Aspliden",
        ],
    },
    Route {
        slinga: "8",
        weekday: Weekday::Mon,
        parity: Parity::Even,
        areas: &[
            "Arvidsjaur", "Långudden", "Fälloheden", "Allejaure", "Lövlund",
            "Radnejaur", "Kålmis", "Lundegård", "Kurrekveik", "Vaxholm/Ånge",
            "Långviken", "Mellanström", "Maddaur", "Sjulnäs", "Nyliden",
            "Reback", "Slagnäs", "Gullön", "Bergnäsudden", "Fiskträsk",
            "Avaviken", "Renviken", "Månsträsk", "Kläppen", "Tallmo",
        ],
    },
    Route {
        slinga: "9",
        weekday: Weekday::Tue,
        parity: Parity::Even,
        areas: &[
            "Arvidsjaur område 3", "Grästjärn", "Tjappsåive", "Moskosel samhälle",
        ],
    },
    Route {
        slinga: "10",
        weekday: Weekday::Wed,
        parity: Parity::Even,
        areas: &["Arjeplog område 1", "Baktåive", "Norrmalm", "Övre långträsk"],
    },
    Route {
        slinga: "11",
        weekday: Weekday::Thu,
        parity: Parity::Even,
        areas: &[
            "Arjeplog", "Lippiudden", "Racksund", "Båtsjaur", "Laisvallby",
            "Laisvall", "Adolfström", "Loholm",
        ],
    },
    Route {
        slinga: "12",
        weekday: Weekday::Thu,
        parity: Parity::Even,
        areas: &[
            "Arjeplog", "Silvervägen till Norska gränsen", "Vouggatjålme",
            "Tjärnberg", "Laisvik", "Bougt",
        ],
    },
    Route {
        slinga: "13",
        weekday: Weekday::Fri,
        parity: Parity::Even,
        areas: &["Glommersträsk samhälle", "Svedjan", "Arvidsjaur område 2"],
    },
];

pub struct Arvidsjaur;

impl Arvidsjaur {
    pub fn new() -> Self {
        Self
    }
}

fn weekday_sv(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "måndag",
        Weekday::Tue => "tisdag",
        Weekday::Wed => "onsdag",
        Weekday::Thu => "torsdag",
        Weekday::Fri => "fredag",
        Weekday::Sat => "lördag",
        Weekday::Sun => "söndag",
    }
}

fn parity_sv(p: Parity) -> &'static str {
    match p {
        Parity::Odd => "udda vecka",
        Parity::Even => "jämn vecka",
    }
}

fn route_label(r: &Route) -> String {
    format!(
        "Slinga {} — {} {} ({})",
        r.slinga,
        weekday_sv(r.weekday),
        parity_sv(r.parity),
        r.areas.join(", "),
    )
}

fn find_route(address: &str) -> Option<&'static Route> {
    let rest = address.trim().strip_prefix("Slinga ")?;
    let id_end = rest
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(rest.len());
    let id = &rest[..id_end];
    ROUTES.iter().find(|r| r.slinga == id)
}

fn matches_parity(date: NaiveDate, parity: Parity) -> bool {
    let week = date.iso_week().week();
    match parity {
        Parity::Odd => week % 2 == 1,
        Parity::Even => week % 2 == 0,
    }
}

/// Genererar ~26 datum (ett år framåt) för `route`, med start >= `today`.
/// Kontrollerar parity efter varje +14 dagar, vilket hanterar 53-veckors-år
/// där ISO-vecko-pariteten flippar över årsskiftet.
fn generate_dates(route: &Route, today: NaiveDate) -> Vec<NaiveDate> {
    let target = route.weekday.num_days_from_monday() as i64;
    let current = today.weekday().num_days_from_monday() as i64;
    let days_forward = (target - current).rem_euclid(7);
    let mut date = today + Duration::days(days_forward);
    while !matches_parity(date, route.parity) {
        date += Duration::days(7);
    }

    let mut dates = Vec::with_capacity(27);
    for _ in 0..27 {
        dates.push(date);
        date += Duration::days(14);
        if !matches_parity(date, route.parity) {
            date += Duration::days(7);
        }
    }
    dates
}

fn build_schedule(route: &Route, today: NaiveDate, requested_address: &str) -> PickupSchedule {
    let dates = generate_dates(route, today);
    let frequency = format!(
        "Varannan vecka — {} {}",
        weekday_sv(route.weekday),
        parity_sv(route.parity),
    );
    PickupSchedule {
        address: requested_address.to_string(),
        series: vec![PickupSeries {
            waste_type: "Hushållsavfall".to_string(),
            frequency_text: frequency,
            interval_weeks: None,
            anchor: dates,
        }],
    }
}

#[async_trait]
impl Provider for Arvidsjaur {
    fn id(&self) -> &'static str {
        "arvidsjaur"
    }
    fn name(&self) -> &'static str {
        "Arvidsjaur"
    }
    fn placeholder(&self) -> &'static str {
        "t.ex. Fjällbonäs, Arvidsjaur område 1 eller Slinga 5"
    }
    fn note(&self) -> &'static str {
        "Arvidsjaurs kommun publicerar hämtningsschemat som statiska \
         slingor, inte per adress. Välj din by/område eller din slinga. \
         Arvidsjaurs tätort är uppdelad i område 1–4 som endast är \
         utmärkta på kommunens karta — slå upp ditt område där och välj \
         motsvarande slinga (område 1 → Slinga 4, område 2 → Slinga 13, \
         område 3 → Slinga 9, område 4 → Slinga 6). För fritidshus finns \
         ett sommarabonnemang vecka 20–43; kontakta Avfallsenheten för att \
         ansluta din fastighet."
    }

    async fn autocomplete(&self, query: &str) -> Result<Vec<Suggestion>, ProviderError> {
        let q = query.trim().to_lowercase();
        if q.len() < 2 {
            return Ok(vec![]);
        }
        let mut out = Vec::new();
        for r in ROUTES {
            let label = route_label(r);
            if label.to_lowercase().contains(&q) {
                out.push(Suggestion { value: label });
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let Some(route) = find_route(address) else {
            return Ok(PickupSchedule {
                address: address.to_string(),
                series: vec![],
            });
        };
        let today = chrono::Local::now().date_naive();
        Ok(build_schedule(route, today, address))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(slinga: &str) -> &'static Route {
        ROUTES.iter().find(|r| r.slinga == slinga).unwrap()
    }

    #[test]
    fn route_table_matches_source() {
        assert_eq!(ROUTES.len(), 13);
        assert_eq!(route("1").weekday, Weekday::Mon);
        assert_eq!(route("1").parity, Parity::Odd);
        assert_eq!(route("5").weekday, Weekday::Thu);
        assert_eq!(route("5").parity, Parity::Odd);
        assert_eq!(route("8").weekday, Weekday::Mon);
        assert_eq!(route("8").parity, Parity::Even);
        assert_eq!(route("13").weekday, Weekday::Fri);
        assert_eq!(route("13").parity, Parity::Even);
    }

    #[test]
    fn find_route_by_slinga_prefix() {
        assert_eq!(find_route("Slinga 1").unwrap().slinga, "1");
        assert_eq!(find_route("Slinga 13").unwrap().slinga, "13");
        assert_eq!(
            find_route("Slinga 7 — fredag udda vecka (Arvidsjaur, ...)")
                .unwrap()
                .slinga,
            "7"
        );
    }

    #[test]
    fn find_route_rejects_unknown() {
        assert!(find_route("Storgatan 1").is_none());
        assert!(find_route("Slinga 99").is_none());
        assert!(find_route("Slinga 11A").is_none());
    }

    #[test]
    fn label_lists_areas() {
        let l = route_label(route("1"));
        assert!(l.starts_with("Slinga 1 — måndag udda vecka"));
        assert!(l.contains("Fjällbonäs"));
        assert!(l.contains("Akkavare"));
    }

    #[test]
    fn generate_dates_hits_correct_weekday_and_parity() {
        // 2026-09-12 is a Saturday. Slinga 1 (Mon odd): next matching Monday
        // is 2026-09-14 (week 38 = even) → skip → 2026-09-21 (week 39 = odd).
        let today = NaiveDate::from_ymd_opt(2026, 9, 12).unwrap();
        let dates = generate_dates(route("1"), today);
        assert_eq!(dates.len(), 27);
        assert_eq!(dates[0], NaiveDate::from_ymd_opt(2026, 9, 21).unwrap());
        for d in &dates {
            assert_eq!(d.weekday(), Weekday::Mon);
            assert_eq!(d.iso_week().week() % 2, 1);
        }
    }

    #[test]
    fn generate_dates_survives_53_week_year_boundary() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        for r in ROUTES {
            let dates = generate_dates(r, today);
            for d in dates {
                assert_eq!(d.weekday(), r.weekday, "route {}", r.slinga);
                let want = match r.parity {
                    Parity::Odd => 1,
                    Parity::Even => 0,
                };
                assert_eq!(d.iso_week().week() % 2, want, "route {} date {}", r.slinga, d);
            }
        }
    }

    #[tokio::test]
    async fn autocomplete_matches_by_area() {
        let p = Arvidsjaur::new();
        let hits = p.autocomplete("moskosel").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].value.starts_with("Slinga 9"));
    }

    #[tokio::test]
    async fn autocomplete_matches_by_slinga_number() {
        let p = Arvidsjaur::new();
        let hits = p.autocomplete("slinga 13").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].value.starts_with("Slinga 13"));
    }

    #[tokio::test]
    async fn autocomplete_ignores_short_queries() {
        let p = Arvidsjaur::new();
        assert!(p.autocomplete("a").await.unwrap().is_empty());
        assert!(p.autocomplete("").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn schedule_returns_empty_for_unknown_input() {
        let p = Arvidsjaur::new();
        let s = p.schedule("Storgatan 1").await.unwrap();
        assert!(s.series.is_empty());
    }
}
