use chrono::{NaiveDate, Utc};
use sha2::{Digest, Sha256};

use crate::providers::{PickupSchedule, PickupSeries};

pub fn build_calendar(kommun: &str, schedule: &PickupSchedule) -> String {
    let mut out = String::new();
    out.push_str("BEGIN:VCALENDAR\r\n");
    out.push_str("VERSION:2.0\r\n");
    out.push_str("PRODID:-//motrice//sopor//SV\r\n");
    out.push_str("CALSCALE:GREGORIAN\r\n");
    out.push_str("METHOD:PUBLISH\r\n");
    out.push_str(&fold(&format!(
        "X-WR-CALNAME:Sophämtning - {}",
        schedule.address
    )));
    out.push_str("X-WR-TIMEZONE:Europe/Stockholm\r\n");
    out.push_str("REFRESH-INTERVAL;VALUE=DURATION:PT12H\r\n");
    out.push_str("X-PUBLISHED-TTL:PT12H\r\n");

    let dtstamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

    for series in &schedule.series {
        if series.anchor.is_empty() {
            continue;
        }
        match (series.interval_weeks, series.anchor.len()) {
            (Some(weeks), 1) => emit_recurring(
                &mut out,
                kommun,
                &schedule.address,
                series,
                series.anchor[0],
                weeks,
                &dtstamp,
            ),
            _ => {
                for date in &series.anchor {
                    emit_single(
                        &mut out,
                        kommun,
                        &schedule.address,
                        series,
                        *date,
                        &dtstamp,
                    );
                }
            }
        }
    }

    out.push_str("END:VCALENDAR\r\n");
    out
}

fn emit_recurring(
    out: &mut String,
    kommun: &str,
    address: &str,
    series: &PickupSeries,
    date: NaiveDate,
    weeks: u32,
    dtstamp: &str,
) {
    let end = date.succ_opt().unwrap_or(date);
    let uid = stable_uid(kommun, address, &series.waste_type);
    let summary = format!("Sophämtning: {}", series.waste_type);
    let description = description(&series.waste_type, &series.frequency_text);

    out.push_str("BEGIN:VEVENT\r\n");
    out.push_str(&format!("UID:{uid}\r\n"));
    out.push_str(&format!("DTSTAMP:{dtstamp}\r\n"));
    out.push_str(&format!("DTSTART;VALUE=DATE:{}\r\n", date.format("%Y%m%d")));
    out.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", end.format("%Y%m%d")));
    out.push_str(&fold(&format!("SUMMARY:{}", escape_text(&summary))));
    out.push_str(&fold(&format!("LOCATION:{}", escape_text(address))));
    out.push_str(&fold(&format!(
        "DESCRIPTION:{}",
        escape_text(&description)
    )));
    out.push_str("TRANSP:TRANSPARENT\r\n");

    let count = (52 / weeks.max(1)).max(1) + 1;
    out.push_str(&format!(
        "RRULE:FREQ=WEEKLY;INTERVAL={weeks};COUNT={count}\r\n"
    ));
    write_valarm(out, &series.waste_type);
    out.push_str("END:VEVENT\r\n");
}

fn emit_single(
    out: &mut String,
    kommun: &str,
    address: &str,
    series: &PickupSeries,
    date: NaiveDate,
    dtstamp: &str,
) {
    let end = date.succ_opt().unwrap_or(date);
    let uid = stable_uid_dated(kommun, address, &series.waste_type, &date);
    let summary = format!("Sophämtning: {}", series.waste_type);
    let description = description(&series.waste_type, &series.frequency_text);

    out.push_str("BEGIN:VEVENT\r\n");
    out.push_str(&format!("UID:{uid}\r\n"));
    out.push_str(&format!("DTSTAMP:{dtstamp}\r\n"));
    out.push_str(&format!("DTSTART;VALUE=DATE:{}\r\n", date.format("%Y%m%d")));
    out.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", end.format("%Y%m%d")));
    out.push_str(&fold(&format!("SUMMARY:{}", escape_text(&summary))));
    out.push_str(&fold(&format!("LOCATION:{}", escape_text(address))));
    out.push_str(&fold(&format!(
        "DESCRIPTION:{}",
        escape_text(&description)
    )));
    out.push_str("TRANSP:TRANSPARENT\r\n");
    write_valarm(out, &series.waste_type);
    out.push_str("END:VEVENT\r\n");
}

fn write_valarm(out: &mut String, waste_type: &str) {
    out.push_str("BEGIN:VALARM\r\n");
    out.push_str("ACTION:DISPLAY\r\n");
    out.push_str(&fold(&format!(
        "DESCRIPTION:{}",
        escape_text(&format!("Ställ ut: {waste_type}"))
    )));
    out.push_str("TRIGGER:-PT6H\r\n");
    out.push_str("END:VALARM\r\n");
}

fn description(waste_type: &str, frequency: &str) -> String {
    if frequency.is_empty() {
        waste_type.to_string()
    } else {
        format!("{waste_type}\n{frequency}")
    }
}

fn stable_uid(kommun: &str, address: &str, waste_type: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kommun.as_bytes());
    hasher.update(b"|");
    hasher.update(address.as_bytes());
    hasher.update(b"|");
    hasher.update(waste_type.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().take(12).map(|b| format!("{:02x}", b)).collect();
    format!("{hex}@sopor.motrice.se")
}

fn stable_uid_dated(kommun: &str, address: &str, waste_type: &str, date: &NaiveDate) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kommun.as_bytes());
    hasher.update(b"|");
    hasher.update(address.as_bytes());
    hasher.update(b"|");
    hasher.update(waste_type.as_bytes());
    hasher.update(b"|");
    hasher.update(date.format("%Y%m%d").to_string().as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().take(12).map(|b| format!("{:02x}", b)).collect();
    format!("{hex}@sopor.motrice.se")
}

fn escape_text(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(';', "\\;")
        .replace(',', "\\,")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{PickupSchedule, PickupSeries};

    fn schedule_recurring() -> PickupSchedule {
        PickupSchedule {
            address: "Olovslundsvägen 9, Bromma, 167 72".into(),
            series: vec![PickupSeries {
                waste_type: "Restavfall".into(),
                frequency_text: "Var 4:e vecka".into(),
                interval_weeks: Some(4),
                anchor: vec![NaiveDate::from_ymd_opt(2026, 6, 30).unwrap()],
            }],
        }
    }

    fn schedule_explicit() -> PickupSchedule {
        PickupSchedule {
            address: "Trotzgatan 13, Falun 79171".into(),
            series: vec![PickupSeries {
                waste_type: "Matavfall".into(),
                frequency_text: "Varje vecka".into(),
                interval_weeks: None,
                anchor: vec![
                    NaiveDate::from_ymd_opt(2026, 6, 17).unwrap(),
                    NaiveDate::from_ymd_opt(2026, 6, 24).unwrap(),
                ],
            }],
        }
    }

    fn count(haystack: &str, needle: &str) -> usize {
        haystack.matches(needle).count()
    }

    #[test]
    fn recurring_emits_single_vevent_with_rrule() {
        let cal = build_calendar("stockholm", &schedule_recurring());
        assert!(cal.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(cal.contains("END:VCALENDAR\r\n"));
        assert_eq!(count(&cal, "BEGIN:VEVENT"), 1);
        assert!(cal.contains("DTSTART;VALUE=DATE:20260630"));
        assert!(cal.contains("DTEND;VALUE=DATE:20260701"));
        assert!(cal.contains("RRULE:FREQ=WEEKLY;INTERVAL=4;COUNT=14"));
        assert!(cal.contains("BEGIN:VALARM"));
        assert!(cal.contains("TRIGGER:-PT6H"));
    }

    #[test]
    fn explicit_dates_emit_one_vevent_each_without_rrule() {
        let cal = build_calendar("falun", &schedule_explicit());
        assert_eq!(count(&cal, "BEGIN:VEVENT"), 2);
        assert!(cal.contains("DTSTART;VALUE=DATE:20260617"));
        assert!(cal.contains("DTSTART;VALUE=DATE:20260624"));
        assert!(!cal.contains("RRULE:"));
    }

    #[test]
    fn explicit_dates_get_distinct_uids() {
        let cal = build_calendar("falun", &schedule_explicit());
        let uids: Vec<&str> = cal
            .lines()
            .filter_map(|l| l.strip_prefix("UID:"))
            .collect();
        assert_eq!(uids.len(), 2);
        assert_ne!(uids[0], uids[1]);
    }

    #[test]
    fn kommun_id_changes_uid() {
        let a = build_calendar("a", &schedule_recurring());
        let b = build_calendar("b", &schedule_recurring());
        let uid_of = |cal: &str| {
            cal.lines()
                .find_map(|l| l.strip_prefix("UID:"))
                .unwrap()
                .to_string()
        };
        assert_ne!(uid_of(&a), uid_of(&b));
    }

    #[test]
    fn special_characters_in_summary_get_escaped() {
        let mut sched = schedule_recurring();
        sched.series[0].waste_type = "Matavfall, villa".into();
        let cal = build_calendar("stockholm", &sched);
        assert!(cal.contains("SUMMARY:Sophämtning: Matavfall\\, villa"));
    }
}

fn fold(line: &str) -> String {
    let mut out = String::new();
    let bytes = line.as_bytes();
    let mut start = 0;
    let mut first = true;
    while start < bytes.len() {
        let limit = if first { 75 } else { 74 };
        let mut end = (start + limit).min(bytes.len());
        while end < bytes.len() && (bytes[end] & 0b1100_0000) == 0b1000_0000 {
            end -= 1;
        }
        if !first {
            out.push(' ');
        }
        out.push_str(std::str::from_utf8(&bytes[start..end]).unwrap_or(""));
        out.push_str("\r\n");
        start = end;
        first = false;
    }
    out
}
