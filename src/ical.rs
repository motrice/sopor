use chrono::{NaiveDate, Utc};
use sha2::{Digest, Sha256};

use crate::svoa::Schedule;

pub fn build_calendar(address: &str, schedule: &Schedule) -> String {
    let mut out = String::new();
    out.push_str("BEGIN:VCALENDAR\r\n");
    out.push_str("VERSION:2.0\r\n");
    out.push_str("PRODID:-//motrice//sopor//SV\r\n");
    out.push_str("CALSCALE:GREGORIAN\r\n");
    out.push_str("METHOD:PUBLISH\r\n");
    out.push_str(&fold(&format!("X-WR-CALNAME:Sophämtning - {}", address)));
    out.push_str("X-WR-TIMEZONE:Europe/Stockholm\r\n");
    out.push_str("REFRESH-INTERVAL;VALUE=DURATION:PT12H\r\n");
    out.push_str("X-PUBLISHED-TTL:PT12H\r\n");

    let dtstamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

    for (waste_type, entries) in schedule {
        for entry in entries {
            let Some(date) = NaiveDate::parse_from_str(&entry.execution_date, "%Y-%m-%d").ok()
            else {
                continue;
            };
            let interval_weeks = parse_interval_weeks(&entry.fetch_frequency);
            let end = date.succ_opt().unwrap_or(date);

            let uid = stable_uid(address, waste_type);
            let summary = format!("Sophämtning: {}", waste_type);
            let description = format!(
                "{}\n{}\nKälla: Stockholm Vatten och Avfall",
                waste_type, entry.fetch_frequency
            );

            out.push_str("BEGIN:VEVENT\r\n");
            out.push_str(&format!("UID:{}\r\n", uid));
            out.push_str(&format!("DTSTAMP:{}\r\n", dtstamp));
            out.push_str(&format!(
                "DTSTART;VALUE=DATE:{}\r\n",
                date.format("%Y%m%d")
            ));
            out.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", end.format("%Y%m%d")));
            out.push_str(&fold(&format!("SUMMARY:{}", escape_text(&summary))));
            out.push_str(&fold(&format!("LOCATION:{}", escape_text(address))));
            out.push_str(&fold(&format!("DESCRIPTION:{}", escape_text(&description))));
            out.push_str("TRANSP:TRANSPARENT\r\n");
            if let Some(weeks) = interval_weeks {
                // Project ~1 year of recurrences so the calendar shows the series
                // even when offline. Client re-fetches periodically and re-anchors.
                let count = (52 / weeks.max(1)).max(1) + 1;
                out.push_str(&format!(
                    "RRULE:FREQ=WEEKLY;INTERVAL={};COUNT={}\r\n",
                    weeks, count
                ));
            }
            out.push_str("BEGIN:VALARM\r\n");
            out.push_str("ACTION:DISPLAY\r\n");
            out.push_str(&fold(&format!(
                "DESCRIPTION:{}",
                escape_text(&format!("Ställ ut: {}", waste_type))
            )));
            // Trigger 18:00 the day before (-PT6H from midnight at DTSTART)
            out.push_str("TRIGGER:-PT6H\r\n");
            out.push_str("END:VALARM\r\n");
            out.push_str("END:VEVENT\r\n");
        }
    }

    out.push_str("END:VCALENDAR\r\n");
    out
}

fn parse_interval_weeks(freq: &str) -> Option<u32> {
    let lower = freq.to_lowercase();
    if lower.contains("varje vecka") {
        return Some(1);
    }
    if lower.contains("varannan vecka") {
        return Some(2);
    }
    // "Var 3:e vecka", "Var 4:e vecka", "Var 8:e vecka" etc.
    let digits: String = lower
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if !digits.is_empty() && lower.contains("vecka") {
        return digits.parse().ok();
    }
    None
}

fn stable_uid(address: &str, waste_type: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(address.as_bytes());
    hasher.update(b"|");
    hasher.update(waste_type.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().take(12).map(|b| format!("{:02x}", b)).collect();
    format!("{}@sopor.motrice.se", hex)
}

fn escape_text(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(';', "\\;")
        .replace(',', "\\,")
}

// iCalendar line folding: split at 75 octets, continuation lines start with a space.
fn fold(line: &str) -> String {
    let mut out = String::new();
    let bytes = line.as_bytes();
    let mut start = 0;
    let mut first = true;
    while start < bytes.len() {
        let limit = if first { 75 } else { 74 };
        let mut end = (start + limit).min(bytes.len());
        // Don't split inside a UTF-8 char.
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
