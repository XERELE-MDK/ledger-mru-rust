//! Dates ("YYYY-MM-DD" strings) and MRU amount formatting — same rules as the Android app.

use chrono::{Datelike, Local, NaiveDate};

pub const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];
const DOW: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

pub fn today() -> String {
    date_str(Local::now().date_naive())
}

/// Local time as "HH:MM".
pub fn now_hhmm() -> String {
    Local::now().format("%H:%M").to_string()
}

pub fn date_str(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Parses "YYYY-MM-DD" (or "YYYY-MM", day 1).
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    let mut it = s.split('-').map(|p| p.parse::<u32>().ok());
    let y = it.next()?? as i32;
    let m = it.next()??;
    let d = it.next().unwrap_or(Some(1))?;
    NaiveDate::from_ymd_opt(y, m, d)
}

fn ym_parts(ym: &str) -> (i32, u32) {
    let d = parse_date(ym).unwrap_or_else(|| Local::now().date_naive());
    (d.year(), d.month())
}

pub fn ym_shift(ym: &str, off: i32) -> String {
    let (y, m) = ym_parts(ym);
    let idx = y * 12 + (m as i32 - 1) + off;
    format!("{}-{:02}", idx.div_euclid(12), idx.rem_euclid(12) + 1)
}

pub fn ym_long(ym: &str) -> String {
    let (y, m) = ym_parts(ym);
    format!("{} {}", MONTHS[m as usize - 1], y)
}

pub fn ym_short(ym: &str) -> String {
    let (y, m) = ym_parts(ym);
    format!("{} {}", &MONTHS[m as usize - 1][..3], y)
}

pub fn month_abbr(ym: &str) -> &'static str {
    &MONTHS[ym_parts(ym).1 as usize - 1][..3]
}

pub fn month_name(ym: &str) -> &'static str {
    MONTHS[ym_parts(ym).1 as usize - 1]
}

pub fn days_in(ym: &str) -> u32 {
    let (y, m) = ym_parts(ym);
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    NaiveDate::from_ymd_opt(ny, nm, 1).unwrap().pred_opt().unwrap().day()
}

pub fn day_long(ds: &str) -> String {
    match parse_date(ds) {
        Some(d) => format!(
            "{} {} {}",
            DOW[d.weekday().num_days_from_monday() as usize],
            d.day(),
            &MONTHS[d.month0() as usize][..3]
        ),
        None => ds.to_string(),
    }
}

pub fn day_short(ds: &str) -> String {
    match parse_date(ds) {
        Some(d) => format!("{} {}", d.day(), &MONTHS[d.month0() as usize][..3]),
        None => ds.to_string(),
    }
}

fn group(v: u64) -> String {
    let s = v.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Whole MRU, grouped, with a true minus sign: `-1240.4` → `−1,240`.
pub fn f0(n: f64) -> String {
    let v = n.round() as i64;
    format!("{}{}", if v < 0 { "−" } else { "" }, group(v.unsigned_abs()))
}

/// MRU with khoums: always 2 decimals when `dec`, otherwise only when needed.
pub fn f2(n: f64, dec: bool) -> String {
    let cents = (n * 100.0).round() as i64;
    let a = cents.unsigned_abs();
    let (whole, frac) = (a / 100, a % 100);
    let mut s = group(whole);
    if dec {
        s += &format!(".{frac:02}");
    } else if frac != 0 {
        s += &if frac % 10 == 0 { format!(".{}", frac / 10) } else { format!(".{frac:02}") };
    }
    format!("{}{}", if cents < 0 { "−" } else { "" }, s)
}

/// Largest amount accepted (same limit as the Android app).
pub const MAX_AMOUNT: f64 = 1e12;

/// Reads what people type, whatever the separators:
/// "1 250,50" · "1,250.50" · "1.250,50" · "1250.5" · "1,250" (= 1250) · "12,5" (= 12.5).
/// Anything else, or more than [`MAX_AMOUNT`], gives 0.
///
/// Same rules as `parseAmount` in the Flutter app, on the same cases
/// (tests/fixtures/amounts.json). Digits and separators only: `f64::from_str`
/// also understands "inf", "NaN" and "1e400", and JSON has no infinity.
pub fn parse_amount(s: &str) -> f64 {
    let mut t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if t.is_empty() || !t.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ',') {
        return 0.0;
    }
    // "1,250" / "12,345,678": 1 to 3 digits, then groups of exactly 3.
    let grouped = |t: &str, sep: char, min_groups: usize| {
        let parts: Vec<&str> = t.split(sep).collect();
        parts.len() > min_groups && (1..=3).contains(&parts[0].len()) && parts[1..].iter().all(|p| p.len() == 3)
    };
    match (t.rfind(','), t.rfind('.')) {
        // Both used: the last one is the decimal mark, the other groups thousands.
        (Some(c), Some(d)) => {
            let (dec, group) = if c > d { (',', '.') } else { ('.', ',') };
            t = t.replace(group, "").replace(dec, ".");
        }
        (Some(_), None) if grouped(&t, ',', 1) => t = t.replace(',', ""),
        (Some(_), None) => t = t.replace(',', "."),
        (None, Some(_)) if grouped(&t, '.', 2) => t = t.replace('.', ""),
        _ => {}
    }
    match t.parse::<f64>() {
        Ok(v) if v.is_finite() && v <= MAX_AMOUNT => v,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(f0(25500.0), "25,500");
        assert_eq!(f0(-1240.4), "−1,240");
        assert_eq!(f2(1250.5, false), "1,250.5");
        assert_eq!(f2(1250.05, false), "1,250.05");
        assert_eq!(f2(1250.0, true), "1,250.00");
        assert_eq!(parse_amount("1 250,50"), 1250.5);
        assert_eq!(parse_amount("abc"), 0.0);
        // Not amounts: an infinite one used to be saved as `null`, and the
        // whole ledger could not be read back.
        for junk in ["inf", "-inf", "infinity", "NaN", "1e400", "1e5", "-5", "+5", "", "2000000000000"] {
            assert_eq!(parse_amount(junk), 0.0, "{junk}");
        }
        assert_eq!(parse_amount("1000000000000"), MAX_AMOUNT);
    }

    /// The same file is read by the Flutter tests.
    #[test]
    fn amounts_match_the_shared_cases() {
        let cases: Vec<(String, f64)> = serde_json::from_str(include_str!("../tests/fixtures/amounts.json")).unwrap();
        assert!(cases.len() > 25);
        for (typed, expected) in cases {
            assert_eq!(parse_amount(&typed), expected, "{typed:?}");
        }
    }

    #[test]
    fn dates() {
        assert_eq!(ym_shift("2026-01", -1), "2025-12");
        assert_eq!(ym_shift("2026-11", 2), "2027-01");
        assert_eq!(days_in("2028-02"), 29);
        assert_eq!(day_long("2026-11-20"), "Fri 20 Nov");
        assert_eq!(ym_long("2026-11"), "November 2026");
    }
}
