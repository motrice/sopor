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
