//! The text `chrono` read as a date or a timestamp, read the same way on `jiff` (CRM-T258).
//!
//! A client that sent a date or a time the service accepted before the move to `jiff` must still be
//! understood, and one it refused must still be refused where the service checks it strictly. These
//! are ports of `chrono` 0.4's own readers, for the forms `jiff` reads differently:
//!
//! - [`date`] is `NaiveDate`'s `FromStr`, which `chrono`'s serde used: a field may go unpadded
//!   (`2026-2-8`), a year outside 0 to 9999 needs a sign, and spaces may come between the parts.
//! - [`date_ymd`] is `NaiveDate::parse_from_str(text, "%Y-%m-%d")`: as [`date`], with no space
//!   before a `-` or at the end.
//! - [`timestamp`] is `DateTime<FixedOffset>`'s `FromStr`, which `chrono`'s serde used: a relaxed
//!   RFC 3339 that also takes unpadded fields, spaces between the parts, `UTC` or an offset without
//!   a colon, and a fraction of more than nine digits, cut to nine.
//! - [`rfc3339`] is `DateTime::parse_from_rfc3339`: strict RFC 3339, with `t`, `z` or a space
//!   allowed, and a fraction of more than nine digits cut to nine.
//!
//! Two things `chrono` read cannot be held: a year beyond 9999 either side, outside `jiff`'s range,
//! and a leap second (`:60`), which is read as `:59`, as `jiff` reads it.
//!
//! Adapted from `chrono` 0.4.45 (`src/format/parse.rs` and `src/format/scan.rs`), used here under
//! its MIT licence. `chrono` is dual-licensed MIT and Apache 2.0, Copyright (c) 2014--2026, Kang
//! Seonghoon and contributors. Its MIT licence:
//!
//! > Copyright (c) 2014, Kang Seonghoon.
//! >
//! > Permission is hereby granted, free of charge, to any person obtaining a copy of this software
//! > and associated documentation files (the "Software"), to deal in the Software without
//! > restriction, including without limitation the rights to use, copy, modify, merge, publish,
//! > distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the
//! > Software is furnished to do so, subject to the following conditions:
//! >
//! > The above copyright notice and this permission notice shall be included in all copies or
//! > substantial portions of the Software.
//! >
//! > THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
//! > BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
//! > NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
//! > DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! > OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use jiff::Timestamp;
use jiff::civil::{Date, DateTime};
use jiff::tz::Offset;

/// From `min` to `max` ASCII digits, as `chrono`'s `scan::number`.
fn number(s: &str, min: usize, max: usize) -> Option<(&str, i64)> {
    let digits = s.bytes().take(max).take_while(u8::is_ascii_digit).count();
    if digits < min {
        return None;
    }
    let mut value: i64 = 0;
    for digit in &s.as_bytes()[..digits] {
        value = value
            .checked_mul(10)?
            .checked_add(i64::from(digit - b'0'))?;
    }
    Some((&s[digits..], value))
}

/// A field after any leading space: one to `width` digits.
fn field(s: &str, width: usize) -> Option<(&str, i64)> {
    number(s.trim_start(), 1, width)
}

/// A year after any leading space: up to four digits, or any number of them after a sign.
fn year(s: &str) -> Option<(&str, i64)> {
    let s = s.trim_start();
    if let Some(rest) = s.strip_prefix('-') {
        number(rest, 1, usize::MAX).map(|(rest, value)| (rest, -value))
    } else if let Some(rest) = s.strip_prefix('+') {
        number(rest, 1, usize::MAX)
    } else {
        number(s, 1, 4)
    }
}

/// `year-month-day`; with `spaced`, a space may also come before each `-`.
fn ymd(s: &str, spaced: bool) -> Option<(&str, [i64; 3])> {
    let (s, y) = year(s)?;
    let s = if spaced { s.trim_start() } else { s };
    let (s, m) = field(s.strip_prefix('-')?, 2)?;
    let s = if spaced { s.trim_start() } else { s };
    let (s, d) = field(s.strip_prefix('-')?, 2)?;
    Some((s, [y, m, d]))
}

fn civil_date([y, m, d]: [i64; 3]) -> Option<Date> {
    Date::new(
        i16::try_from(y).ok()?,
        i8::try_from(m).ok()?,
        i8::try_from(d).ok()?,
    )
    .ok()
}

/// A date as `chrono`'s `NaiveDate::from_str` read one.
pub fn date(text: &str) -> Option<Date> {
    let (rest, ymd) = ymd(text, true)?;
    rest.trim_start().is_empty().then_some(())?;
    civil_date(ymd)
}

/// A date as `chrono`'s `NaiveDate::parse_from_str(text, "%Y-%m-%d")` read one.
pub fn date_ymd(text: &str) -> Option<Date> {
    let (rest, ymd) = ymd(text, false)?;
    rest.is_empty().then_some(())?;
    civil_date(ymd)
}

/// A fraction of a second after its `.`: at least one digit, any after the ninth dropped.
fn nanoseconds(s: &str) -> Option<(&str, i32)> {
    let (rest, value) = number(s, 1, 9)?;
    let scale = 10_i64.pow(9 - u32::try_from(s.len() - rest.len()).ok()?);
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    Some((rest, i32::try_from(value * scale).ok()?))
}

/// An offset as `chrono`'s `scan::timezone_offset` read one for these two readers: `Z`, or a sign
/// (`+`, `-` or U+2212 MINUS SIGN), two digits of hours, then with `relaxed` any colons and spaces
/// and else exactly one colon, and two digits of minutes.
fn offset(s: &str, relaxed: bool) -> Option<(&str, i32)> {
    if let Some(b'Z' | b'z') = s.as_bytes().first() {
        return Some((&s[1..], 0));
    }
    let (negative, s) = if let Some(rest) = s.strip_prefix('+') {
        (false, rest)
    } else if let Some(rest) = s.strip_prefix('-') {
        (true, rest)
    } else {
        (true, s.strip_prefix('\u{2212}')?)
    };
    let two = |s: &str, first: std::ops::RangeInclusive<u8>| -> Option<i32> {
        match *s.as_bytes() {
            [a, b, ..] if first.contains(&a) && b.is_ascii_digit() => {
                Some(i32::from(a - b'0') * 10 + i32::from(b - b'0'))
            }
            _ => None,
        }
    };
    let hours = two(s, b'0'..=b'9')?;
    let s = &s[2..];
    let s = if relaxed {
        s.trim_start_matches(|c: char| c == ':' || c.is_whitespace())
    } else {
        s.strip_prefix(':')?
    };
    let minutes = two(s, b'0'..=b'5')?;
    let seconds = hours * 3600 + minutes * 60;
    Some((&s[2..], if negative { -seconds } else { seconds }))
}

/// The instant, from fields `chrono` would have accepted: an offset under a day, and a leap second
/// read as the second before it.
fn instant(ymd: [i64; 3], hms: [i64; 3], nanos: i32, offset: i32) -> Option<Timestamp> {
    if offset.abs() >= 86_400 {
        return None;
    }
    let [h, mi, s] = hms;
    let s = if s == 60 { 59 } else { s };
    let local = DateTime::new(
        0,
        1,
        1,
        i8::try_from(h).ok()?,
        i8::try_from(mi).ok()?,
        i8::try_from(s).ok()?,
        nanos,
    )
    .ok()?;
    let local = civil_date(ymd)?.to_datetime(local.time());
    Offset::from_seconds(offset).ok()?.to_timestamp(local).ok()
}

/// A timestamp as `chrono`'s `DateTime<FixedOffset>::from_str` read one.
pub fn timestamp(text: &str) -> Option<Timestamp> {
    let (s, ymd) = ymd(text, true)?;
    let s = match s.as_bytes().first()? {
        b't' | b'T' | b' ' => &s[1..],
        _ => return None,
    };
    let (s, h) = field(s, 2)?;
    let (s, mi) = field(s.trim_start().strip_prefix(':')?, 2)?;
    let (s, sec) = field(s.trim_start().strip_prefix(':')?, 2)?;
    let (s, nanos) = match s.strip_prefix('.') {
        Some(rest) => nanoseconds(rest)?,
        None => (s, 0),
    };
    let s = s.trim_start();
    let (s, offset) = if s.len() >= 3 && s.as_bytes()[..3].eq_ignore_ascii_case(b"UTC") {
        (&s[3..], 0)
    } else {
        offset(s, true)?
    };
    s.trim_start().is_empty().then_some(())?;
    instant(ymd, [h, mi, sec], nanos, offset)
}

/// Exactly `width` ASCII digits.
fn fixed(s: &str, width: usize) -> Option<(&str, i64)> {
    s.as_bytes()
        .get(..width)?
        .iter()
        .all(u8::is_ascii_digit)
        .then(|| number(s, width, width))?
}

/// A timestamp as `chrono`'s `DateTime::parse_from_rfc3339` read one.
pub fn rfc3339(text: &str) -> Option<Timestamp> {
    let (s, y) = fixed(text, 4)?;
    let (s, mo) = fixed(s.strip_prefix('-')?, 2)?;
    let (s, d) = fixed(s.strip_prefix('-')?, 2)?;
    let s = match s.as_bytes().first()? {
        b't' | b'T' | b' ' => &s[1..],
        _ => return None,
    };
    let (s, h) = fixed(s, 2)?;
    let (s, mi) = fixed(s.strip_prefix(':')?, 2)?;
    let (s, sec) = fixed(s.strip_prefix(':')?, 2)?;
    let (s, nanos) = match s.strip_prefix('.') {
        Some(rest) => nanoseconds(rest)?,
        None => (s, 0),
    };
    let (s, offset) = offset(s, false)?;
    s.is_empty().then_some(())?;
    instant([y, mo, d], [h, mi, sec], nanos, offset)
}
