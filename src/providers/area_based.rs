//! Delad provider för kommuner som publicerar rutt-baserade
//! hämtningsscheman utan per-adress-uppslag. Varje kommun får en
//! `Config` med en tabell över slingor; varje slinga har veckodag
//! och frekvens (varje vecka / varannan vecka med udda/jämna veckor)
//! plus en lista med områden. Datum genereras lokalt.
//!
//! Motsvarar mönstret i `arjeplog.rs` och `arvidsjaur.rs` men i
//! Config-form så flera kommuner delar en gemensam kodbas.
//! `arjeplog.rs`/`arvidsjaur.rs` behålls separat pga special-features
//! (månadstömning, sommarrutt).

use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate, Weekday};

use super::{PickupSchedule, PickupSeries, Provider, ProviderError, Suggestion};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interval {
    Weekly,
    Biweekly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    Odd,
    Even,
}

#[derive(Debug, Clone, Copy)]
pub struct Route {
    pub slinga: &'static str,
    pub weekday: Weekday,
    pub interval: Interval,
    /// Parity is only meaningful for `Interval::Biweekly`; for weekly
    /// routes any value works and is ignored.
    pub parity: Parity,
    pub areas: &'static [&'static str],
}

pub struct Config {
    pub id: &'static str,
    pub name: &'static str,
    pub placeholder: &'static str,
    pub note: &'static str,
    pub routes: &'static [Route],
}

pub struct AreaBased {
    cfg: Config,
}

impl AreaBased {
    pub fn new(cfg: Config) -> Self {
        Self { cfg }
    }
}

pub fn weekday_sv(w: Weekday) -> &'static str {
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

fn cadence_label(r: &Route) -> String {
    match r.interval {
        Interval::Weekly => format!("varje vecka — {}", weekday_sv(r.weekday)),
        Interval::Biweekly => {
            let parity = match r.parity {
                Parity::Odd => "udda vecka",
                Parity::Even => "jämn vecka",
            };
            format!("varannan vecka — {} ({})", weekday_sv(r.weekday), parity)
        }
    }
}

fn route_label(r: &Route) -> String {
    format!(
        "Slinga {} — {} ({})",
        r.slinga,
        cadence_label(r),
        r.areas.join(", "),
    )
}

fn find_route<'a>(cfg: &'a Config, address: &str) -> Option<&'a Route> {
    let rest = address.trim().strip_prefix("Slinga ")?;
    let id_end = rest
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(rest.len());
    let id = &rest[..id_end];
    cfg.routes.iter().find(|r| r.slinga == id)
}

fn matches_parity(date: NaiveDate, parity: Parity) -> bool {
    let week = date.iso_week().week();
    match parity {
        Parity::Odd => week % 2 == 1,
        Parity::Even => week % 2 == 0,
    }
}

/// Genererar 27 datum framåt för en slinga från och med `today`.
/// Hanterar 53-veckors-år där ISO-vecko-pariteten flippar över årsskiftet.
pub fn generate_dates(route: &Route, today: NaiveDate) -> Vec<NaiveDate> {
    let target = route.weekday.num_days_from_monday() as i64;
    let current = today.weekday().num_days_from_monday() as i64;
    let days_forward = (target - current).rem_euclid(7);
    let mut date = today + Duration::days(days_forward);
    if matches!(route.interval, Interval::Biweekly) {
        while !matches_parity(date, route.parity) {
            date += Duration::days(7);
        }
    }
    let step = match route.interval {
        Interval::Weekly => 7,
        Interval::Biweekly => 14,
    };
    let mut out = Vec::with_capacity(27);
    for _ in 0..27 {
        out.push(date);
        date += Duration::days(step);
        if matches!(route.interval, Interval::Biweekly) && !matches_parity(date, route.parity) {
            date += Duration::days(7);
        }
    }
    out
}

fn build_schedule(route: &Route, today: NaiveDate, requested_address: &str) -> PickupSchedule {
    let dates = generate_dates(route, today);
    let frequency = match route.interval {
        Interval::Weekly => format!("Varje vecka — {}", weekday_sv(route.weekday)),
        Interval::Biweekly => {
            let parity = match route.parity {
                Parity::Odd => "udda vecka",
                Parity::Even => "jämn vecka",
            };
            format!(
                "Varannan vecka — {} ({parity})",
                weekday_sv(route.weekday),
            )
        }
    };
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
impl Provider for AreaBased {
    fn id(&self) -> &'static str {
        self.cfg.id
    }
    fn name(&self) -> &'static str {
        self.cfg.name
    }
    fn placeholder(&self) -> &'static str {
        self.cfg.placeholder
    }
    fn note(&self) -> &'static str {
        self.cfg.note
    }

    async fn autocomplete(&self, query: &str) -> Result<Vec<Suggestion>, ProviderError> {
        let q = query.trim().to_lowercase();
        if q.len() < 2 {
            return Ok(vec![]);
        }
        let mut out = Vec::new();
        for r in self.cfg.routes {
            let label = route_label(r);
            if label.to_lowercase().contains(&q) {
                out.push(Suggestion { value: label });
            }
        }
        Ok(out)
    }

    async fn schedule(&self, address: &str) -> Result<PickupSchedule, ProviderError> {
        let Some(route) = find_route(&self.cfg, address) else {
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

    static ROUTES: &[Route] = &[
        Route {
            slinga: "1",
            weekday: Weekday::Mon,
            interval: Interval::Biweekly,
            parity: Parity::Odd,
            areas: &["Byn", "Kyrkbyn"],
        },
        Route {
            slinga: "2",
            weekday: Weekday::Wed,
            interval: Interval::Weekly,
            parity: Parity::Even,
            areas: &["Centrum"],
        },
    ];

    static CFG: Config = Config {
        id: "test",
        name: "Test",
        placeholder: "t.ex. Byn",
        note: "test",
        routes: ROUTES,
    };

    #[test]
    fn find_route_by_slinga_prefix() {
        assert_eq!(find_route(&CFG, "Slinga 1").unwrap().slinga, "1");
        assert_eq!(
            find_route(&CFG, "Slinga 2 — onsdag varje vecka (Centrum)").unwrap().slinga,
            "2"
        );
        assert!(find_route(&CFG, "Storgatan 1").is_none());
        assert!(find_route(&CFG, "Slinga 99").is_none());
    }

    #[test]
    fn route_label_biweekly_includes_parity() {
        let label = route_label(&ROUTES[0]);
        assert!(label.starts_with("Slinga 1 — varannan vecka — måndag (udda vecka)"));
        assert!(label.contains("Byn"));
    }

    #[test]
    fn route_label_weekly_has_no_parity() {
        let label = route_label(&ROUTES[1]);
        assert!(label.starts_with("Slinga 2 — varje vecka — onsdag"));
        assert!(!label.contains("udda") && !label.contains("jämn"));
    }

    #[test]
    fn generate_dates_biweekly_odd_monday() {
        // 2026-09-12 is Sat; first matching Mon-odd is 2026-09-21 (week 39 odd).
        let today = NaiveDate::from_ymd_opt(2026, 9, 12).unwrap();
        let dates = generate_dates(&ROUTES[0], today);
        assert_eq!(dates.len(), 27);
        assert_eq!(dates[0], NaiveDate::from_ymd_opt(2026, 9, 21).unwrap());
        for d in &dates {
            assert_eq!(d.weekday(), Weekday::Mon);
            assert_eq!(d.iso_week().week() % 2, 1);
        }
    }

    #[test]
    fn generate_dates_weekly_ignores_parity() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 12).unwrap();
        let dates = generate_dates(&ROUTES[1], today);
        assert_eq!(dates.len(), 27);
        assert_eq!(dates[0], NaiveDate::from_ymd_opt(2026, 9, 16).unwrap());
        for w in dates.windows(2) {
            assert_eq!((w[1] - w[0]).num_days(), 7);
        }
    }

    #[test]
    fn generate_dates_survives_53_week_year_boundary() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        for r in ROUTES {
            let dates = generate_dates(r, today);
            for d in dates {
                assert_eq!(d.weekday(), r.weekday, "route {}", r.slinga);
                if matches!(r.interval, Interval::Biweekly) {
                    let want = match r.parity {
                        Parity::Odd => 1,
                        Parity::Even => 0,
                    };
                    assert_eq!(d.iso_week().week() % 2, want, "route {}", r.slinga);
                }
            }
        }
    }

    #[tokio::test]
    async fn autocomplete_matches_by_area() {
        let p = AreaBased::new(Config { routes: ROUTES, ..CFG });
        let hits = p.autocomplete("byn").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].value.starts_with("Slinga 1"));
    }

    #[tokio::test]
    async fn autocomplete_matches_by_slinga_number() {
        let p = AreaBased::new(Config { routes: ROUTES, ..CFG });
        let hits = p.autocomplete("slinga 2").await.unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].value.starts_with("Slinga 2"));
    }

    #[tokio::test]
    async fn autocomplete_ignores_short_queries() {
        let p = AreaBased::new(Config { routes: ROUTES, ..CFG });
        assert!(p.autocomplete("a").await.unwrap().is_empty());
        assert!(p.autocomplete("").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn schedule_returns_empty_for_unknown_input() {
        let p = AreaBased::new(Config { routes: ROUTES, ..CFG });
        let s = p.schedule("Storgatan 1").await.unwrap();
        assert!(s.series.is_empty());
    }
}
