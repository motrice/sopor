//! Arjeplog — statisk ruttbaserad kalender. Kommunen publicerar inte
//! adress-uppslag; hämtningen är istället organiserad i 9 slingor
//! (Slinga 2, 3, 4, 6, 8, 10, 11, 12, 11A) med fast veckodag + jämn/udda
//! ISO-vecka. Denna provider läser inget externt — rutt-tabellen är
//! transkriberad från arjeplog.se och datumen genereras lokalt.
//!
//! Slinga 11A är märkt "Sommarrutt" utan explicit angiven veckodag. Den
//! ligger dock listad direkt efter Slinga 11 och 12 (båda torsdagar
//! jämna veckor), så den antas köras torsdag jämn vecka under sommaren.
//! Säsongens start- och slutdatum framgår inte av kommunens sida —
//! feeden emitterar året runt och användaren får tolka kontexten.
//!
//! Källa (verifierad 2026-09-12):
//! https://arjeplog.se/bygga-bo-och-miljo/avfall-och-atervinning/hamtning/

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
    monthly_areas: &'static [&'static str],
    /// Sommarrutt utan publicerat säsongsintervall. Visas med varning i
    /// autocomplete och i kalenderposternas beskrivning.
    summer_only: bool,
}

const ROUTES: &[Route] = &[
    Route {
        slinga: "2",
        weekday: Weekday::Tue,
        parity: Parity::Odd,
        areas: &[
            "Björkliden", "Glottje", "Kronogård", "Nuortejaur", "Utterbacken",
            "Strittjomvare", "Östansjö", "Strömnäs", "Suddesjaure", "Brännudden",
            "Norrstrand", "Lappvallen", "Lövudden", "Rebnis", "Siksele",
            "Galtisjaure", "Stensund", "Svannäs",
        ],
        monthly_areas: &[
            "Abraure", "Gitton", "Varasviken", "Rävudden",
            "Norra Bergnäs", "Örnvik",
        ],
        summer_only: false,
    },
    Route {
        slinga: "3",
        weekday: Weekday::Tue,
        parity: Parity::Even,
        areas: &[
            "Dainak", "Geigo", "Vaxborg", "Maskaure", "Gransel",
            "Innervik", "Racksund",
        ],
        monthly_areas: &[],
        summer_only: false,
    },
    Route {
        slinga: "4",
        weekday: Weekday::Wed,
        parity: Parity::Even,
        areas: &["Arjeplog Torget", "Hamnen", "Lugnet", "Norrbacka"],
        monthly_areas: &[],
        summer_only: false,
    },
    Route {
        slinga: "6",
        weekday: Weekday::Thu,
        parity: Parity::Odd,
        areas: &["Arjeplog Öberget"],
        monthly_areas: &[],
        summer_only: false,
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
        monthly_areas: &[],
        summer_only: false,
    },
    Route {
        slinga: "10",
        weekday: Weekday::Wed,
        parity: Parity::Even,
        areas: &["Baktåive", "Norrmalm", "Övre långträsk"],
        monthly_areas: &[],
        summer_only: false,
    },
    Route {
        slinga: "11",
        weekday: Weekday::Thu,
        parity: Parity::Even,
        areas: &[
            "Lippiudden", "Racksund", "Båtsjaur", "Laisvallby", "Laisvall",
            "Adolfström", "Loholm",
        ],
        monthly_areas: &[],
        summer_only: false,
    },
    Route {
        slinga: "12",
        weekday: Weekday::Thu,
        parity: Parity::Even,
        areas: &[
            "Silvervägen till Norska gränsen", "Arjeplog", "Vouggatjålme",
            "Tjärnberg", "Laisvik", "Bougt",
        ],
        monthly_areas: &[],
        summer_only: false,
    },
    // 11A saknar publicerad veckodag/parity. Antagandet torsdag jämn
    // vecka bygger på att kommunen listar den direkt efter Slinga 11 och
    // 12 (båda torsdag jämn vecka). Säsongen är inte publicerad.
    Route {
        slinga: "11A",
        weekday: Weekday::Thu,
        parity: Parity::Even,
        areas: &["Se karta 2 på arjeplog.se"],
        monthly_areas: &[],
        summer_only: true,
    },
];

pub struct Arjeplog;

impl Arjeplog {
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
    let day_part = if r.summer_only {
        format!(
            "sommarrutt ({} {} enligt kommunens listordning; säsong ej publicerad)",
            weekday_sv(r.weekday),
            parity_sv(r.parity),
        )
    } else {
        format!("{} {}", weekday_sv(r.weekday), parity_sv(r.parity))
    };
    let mut label = format!(
        "Slinga {} — {} ({}",
        r.slinga,
        day_part,
        r.areas.join(", "),
    );
    if !r.monthly_areas.is_empty() {
        label.push_str("; månadstömning: ");
        label.push_str(&r.monthly_areas.join(", "));
    }
    label.push(')');
    label
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
    let (waste_type, frequency) = if route.summer_only {
        (
            "Hushållsavfall (sommarrutt)".to_string(),
            format!(
                "Sommarrutt — antaget {} {} (osäkert; säsong ej publicerad)",
                weekday_sv(route.weekday),
                parity_sv(route.parity),
            ),
        )
    } else {
        (
            "Hushållsavfall".to_string(),
            format!(
                "Varannan vecka — {} {}",
                weekday_sv(route.weekday),
                parity_sv(route.parity),
            ),
        )
    };
    PickupSchedule {
        address: requested_address.to_string(),
        series: vec![PickupSeries {
            waste_type,
            frequency_text: frequency,
            interval_weeks: None,
            anchor: dates,
        }],
    }
}

#[async_trait]
impl Provider for Arjeplog {
    fn id(&self) -> &'static str {
        "arjeplog"
    }
    fn name(&self) -> &'static str {
        "Arjeplog"
    }
    fn placeholder(&self) -> &'static str {
        "t.ex. Kronogård, Öberget eller Slinga 4"
    }
    fn note(&self) -> &'static str {
        "Arjeplogs kommun publicerar hämtningsschemat som statiska rutter, \
         inte per adress. Välj din by/område eller din slinga. Byar i \
         Slinga 2 markerade som \"månadstömning\" (Abraure, Gitton, \
         Varasviken, Rävudden, Norra Bergnäs, Örnvik) hämtas endast en \
         gång per månad — kontakta kommunen för exakt datum. Slinga 11A \
         är sommarrutt och saknar publicerad veckodag och säsong; den \
         antas köras torsdag jämn vecka (utifrån kommunens listordning \
         direkt efter Slinga 11 och 12) — verifiera med kommunen."
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
        assert_eq!(ROUTES.len(), 9);
        assert_eq!(route("2").weekday, Weekday::Tue);
        assert_eq!(route("2").parity, Parity::Odd);
        assert_eq!(route("4").weekday, Weekday::Wed);
        assert_eq!(route("4").parity, Parity::Even);
        assert_eq!(route("6").weekday, Weekday::Thu);
        assert_eq!(route("6").parity, Parity::Odd);
        assert_eq!(route("8").weekday, Weekday::Mon);
        assert_eq!(route("8").parity, Parity::Even);
        assert!(route("11A").summer_only);
        assert!(!route("11").summer_only);
    }

    #[test]
    fn find_route_by_slinga_prefix() {
        assert_eq!(find_route("Slinga 4").unwrap().slinga, "4");
        assert_eq!(find_route("Slinga 4 — onsdag jämn vecka").unwrap().slinga, "4");
        assert_eq!(find_route("Slinga 11").unwrap().slinga, "11");
        assert_eq!(find_route("Slinga 11A").unwrap().slinga, "11A");
        assert_eq!(find_route("Slinga 11A — sommarrutt").unwrap().slinga, "11A");
    }

    #[test]
    fn find_route_rejects_unknown() {
        assert!(find_route("Storgatan 1").is_none());
        assert!(find_route("Slinga 99").is_none());
    }

    #[test]
    fn summer_route_label_flags_ambiguity() {
        let l = route_label(route("11A"));
        assert!(l.contains("sommarrutt"));
        assert!(l.contains("torsdag jämn vecka"));
        assert!(l.contains("säsong ej publicerad"));
    }

    #[test]
    fn summer_route_schedule_flags_ambiguity() {
        let today = NaiveDate::from_ymd_opt(2026, 6, 30).unwrap();
        let s = build_schedule(route("11A"), today, "Slinga 11A");
        assert_eq!(s.series.len(), 1);
        assert!(s.series[0].waste_type.contains("sommarrutt"));
        assert!(s.series[0].frequency_text.contains("Sommarrutt"));
        assert!(s.series[0].frequency_text.contains("osäkert"));
    }

    #[test]
    fn label_includes_areas_and_monthly_marker() {
        let l = route_label(route("2"));
        assert!(l.starts_with("Slinga 2 — tisdag udda vecka"));
        assert!(l.contains("Kronogård"));
        assert!(l.contains("månadstömning"));
        assert!(l.contains("Abraure"));

        let l4 = route_label(route("4"));
        assert!(l4.starts_with("Slinga 4 — onsdag jämn vecka"));
        assert!(l4.contains("Torget"));
        assert!(!l4.contains("månadstömning"));
    }

    #[test]
    fn generate_dates_hits_correct_weekday_and_parity() {
        // 2026-09-12 is a Saturday. First matching Tuesday-odd-week is
        // 2026-09-15 (week 38 = even) → skip → 2026-09-22 (week 39 = odd).
        let today = NaiveDate::from_ymd_opt(2026, 9, 12).unwrap();
        let dates = generate_dates(route("2"), today);
        assert_eq!(dates.len(), 27);
        assert_eq!(dates[0], NaiveDate::from_ymd_opt(2026, 9, 22).unwrap());
        for d in &dates {
            assert_eq!(d.weekday(), Weekday::Tue);
            assert_eq!(d.iso_week().week() % 2, 1);
        }
    }

    #[test]
    fn generate_dates_starts_on_or_after_today() {
        // 2026-06-30 is a Tuesday, ISO week 27 (odd). Should be picked as-is.
        let today = NaiveDate::from_ymd_opt(2026, 6, 30).unwrap();
        let dates = generate_dates(route("2"), today);
        assert_eq!(dates[0], today);
    }

    #[test]
    fn generate_dates_survives_53_week_year_boundary() {
        // 2026 has 53 ISO weeks. Bi-weekly INTERVAL=2 without correction
        // would flip parity across the boundary. Verify all generated
        // dates keep the target parity.
        let today = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        for r in ROUTES {
            let dates = generate_dates(r, today);
            for d in dates {
                assert_eq!(d.weekday(), r.weekday, "route {}", r.slinga);
                let expected_parity = d.iso_week().week() % 2;
                let want = match r.parity {
                    Parity::Odd => 1,
                    Parity::Even => 0,
                };
                assert_eq!(expected_parity, want, "route {} date {}", r.slinga, d);
            }
        }
    }

    #[tokio::test]
    async fn autocomplete_matches_by_area() {
        let p = Arjeplog::new();
        let hits = p.autocomplete("kronogård").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].value.starts_with("Slinga 2"));
    }

    #[tokio::test]
    async fn autocomplete_matches_by_slinga_number() {
        let p = Arjeplog::new();
        let hits = p.autocomplete("slinga 8").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].value.starts_with("Slinga 8"));
    }

    #[tokio::test]
    async fn autocomplete_ignores_short_queries() {
        let p = Arjeplog::new();
        assert!(p.autocomplete("a").await.unwrap().is_empty());
        assert!(p.autocomplete("").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn schedule_returns_empty_for_unknown_input() {
        let p = Arjeplog::new();
        let s = p.schedule("Storgatan 1").await.unwrap();
        assert!(s.series.is_empty());
    }
}
