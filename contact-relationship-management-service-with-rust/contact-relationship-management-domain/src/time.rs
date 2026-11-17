//! The service's instants and dates, on `jiff` (CRM-T258), as Postgres and JSON carry them.
//!
//! `sqlx` 0.9 reads and writes dates as `chrono` or `time` values only, so [`Timestamp`] and
//! [`Date`] wrap `jiff`'s own types to read a `timestamptz` and a `date` column, and `sqlx.toml`
//! names them for every such column the query macros see. Both serialize as the `chrono` types
//! they replace did, so no API response changes: a timestamp is RFC 3339 in UTC with `Z`, and its
//! fraction, when it has one, in three, six or nine digits; a date is `YYYY-MM-DD`.
//! [`Timestamp::to_rfc3339`] writes `+00:00` instead, as `chrono`'s method of that name did.
//!
//! Both also read what `chrono` read from a client ([`chrono_text`]), so a request that was
//! understood before is understood the same way.
//!
//! The pure rules use `jiff`'s types directly where nothing they hold is stored or sent; these
//! wrappers are for what is.

mod chrono_text;
#[cfg(feature = "postgres")]
mod postgres;

use std::{fmt, str::FromStr};

use jiff::{SignedDuration, civil};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Postgres counts timestamps in microseconds, and dates in days, from 2000-01-01.
#[cfg(any(test, feature = "postgres"))]
const PG_EPOCH_SECONDS: i64 = 946_684_800;
#[cfg(any(test, feature = "postgres"))]
const PG_EPOCH_DATE: civil::Date = civil::date(2000, 1, 1);

/// An instant, as a `timestamptz` column holds one: to the microsecond, with no zone of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub jiff::Timestamp);

impl Timestamp {
    /// The instant now, on this process's clock. The database's `now()` is the clock records are
    /// compared on (CRM-T248); this is for what never meets a stored time.
    #[must_use]
    pub fn now() -> Self {
        Self(jiff::Timestamp::now())
    }

    /// Strict RFC 3339, as `chrono`'s `DateTime::parse_from_rfc3339` read it: `None` for anything
    /// else, such as a time without seconds or an offset without a colon.
    #[must_use]
    pub fn parse_rfc3339(text: &str) -> Option<Self> {
        chrono_text::rfc3339(text).map(Self)
    }

    /// The instant `seconds` after 1970-01-01T00:00:00Z.
    ///
    /// # Errors
    ///
    /// When the instant is outside `jiff`'s range, years -9999 to 9999.
    pub fn from_second(seconds: i64) -> Result<Self, jiff::Error> {
        jiff::Timestamp::from_second(seconds).map(Self)
    }

    /// Whole seconds since 1970-01-01T00:00:00Z, rounded toward zero. `chrono`'s `timestamp()`
    /// rounded down; the two differ only before 1970.
    #[must_use]
    pub fn as_second(self) -> i64 {
        self.0.as_second()
    }

    /// Microseconds since 1970-01-01T00:00:00Z, rounded toward zero.
    #[must_use]
    pub fn as_microsecond(self) -> i64 {
        self.0.as_microsecond()
    }

    /// The day this instant falls on in UTC.
    #[must_use]
    pub fn date_utc(self) -> Date {
        Date(self.0.to_zoned(jiff::tz::TimeZone::UTC).date())
    }

    /// The clock time in UTC at this instant.
    #[must_use]
    pub fn utc(self) -> civil::DateTime {
        jiff::tz::TimeZone::UTC.to_datetime(self.0)
    }

    /// This instant in UTC written with `strftime` conversions, as `chrono`'s `format` wrote a UTC
    /// time with the same ones (`%Y%m%dT%H%M%SZ`, `%a, %d %b %Y %H:%M:%S GMT`).
    #[must_use]
    pub fn strftime_utc<F: AsRef<[u8]> + ?Sized>(
        self,
        format: &F,
    ) -> jiff::fmt::strtime::Display<'_> {
        self.0.to_zoned(jiff::tz::TimeZone::UTC).strftime(format)
    }

    /// The instant `by` later, or earlier for a negative `by`; `None` outside `jiff`'s range.
    #[must_use]
    pub fn checked_add(self, by: SignedDuration) -> Option<Self> {
        self.0.checked_add(by).ok().map(Self)
    }

    /// The time from `earlier` to this instant, negative when `earlier` is later.
    #[must_use]
    pub fn duration_since(self, earlier: Self) -> SignedDuration {
        self.0.duration_since(earlier.0)
    }

    /// RFC 3339, as `chrono`'s `to_rfc3339` wrote a UTC instant: `+00:00`, and a fraction only
    /// when there is one, in three, six or nine digits.
    #[must_use]
    pub fn to_rfc3339(self) -> String {
        self.rfc3339("+00:00")
    }

    /// RFC 3339 as `chrono`'s serde wrote a UTC instant: as [`Self::to_rfc3339`], with `Z` for the
    /// offset.
    #[must_use]
    pub fn to_rfc3339_z(self) -> String {
        self.rfc3339("Z")
    }

    fn rfc3339(self, offset: &str) -> String {
        let zoned = self.0.to_zoned(jiff::tz::TimeZone::UTC);
        let nanos = self.0.subsec_nanosecond();
        let fraction = if nanos == 0 {
            String::new()
        } else if nanos % 1_000_000 == 0 {
            format!(".{:03}", nanos / 1_000_000)
        } else if nanos % 1_000 == 0 {
            format!(".{:06}", nanos / 1_000)
        } else {
            format!(".{nanos:09}")
        };
        format!("{}{fraction}{offset}", zoned.strftime("%Y-%m-%dT%H:%M:%S"))
    }

    /// Microseconds from Postgres's epoch, as a `timestamptz` is sent and stored. A fraction finer
    /// than a microsecond is dropped, as Postgres would round it away.
    #[cfg(any(test, feature = "postgres"))]
    pub(crate) fn to_pg_micros(self) -> Result<i64, String> {
        self.0
            .as_microsecond()
            .checked_sub(PG_EPOCH_SECONDS * 1_000_000)
            .ok_or_else(|| "timestamp out of range for Postgres".to_string())
    }

    #[cfg(any(test, feature = "postgres"))]
    pub(crate) fn from_pg_micros(micros: i64) -> Result<Self, String> {
        let unix = micros
            .checked_add(PG_EPOCH_SECONDS * 1_000_000)
            .ok_or("timestamp out of range")?;
        jiff::Timestamp::from_microsecond(unix)
            .map(Self)
            .map_err(|e| e.to_string())
    }
}

impl From<jiff::Timestamp> for Timestamp {
    fn from(value: jiff::Timestamp) -> Self {
        Self(value)
    }
}

impl From<Timestamp> for jiff::Timestamp {
    fn from(value: Timestamp) -> Self {
        value.0
    }
}

impl std::ops::Add<SignedDuration> for Timestamp {
    type Output = Self;

    /// # Panics
    ///
    /// When the result is outside `jiff`'s range, as `chrono`'s `+` did outside its own.
    fn add(self, by: SignedDuration) -> Self {
        Self(self.0 + by)
    }
}

impl std::ops::Sub<SignedDuration> for Timestamp {
    type Output = Self;

    /// # Panics
    ///
    /// When the result is outside `jiff`'s range.
    fn sub(self, by: SignedDuration) -> Self {
        Self(self.0 - by)
    }
}

impl std::ops::Sub for Timestamp {
    type Output = SignedDuration;

    /// The time from `earlier` to this instant, as [`Timestamp::duration_since`].
    fn sub(self, earlier: Self) -> SignedDuration {
        self.duration_since(earlier)
    }
}

impl std::ops::AddAssign<SignedDuration> for Timestamp {
    fn add_assign(&mut self, by: SignedDuration) {
        *self = *self + by;
    }
}

/// `n` days of 24 hours, as `chrono::Duration::days` counted them.
///
/// # Panics
///
/// When `n` days do not fit a [`SignedDuration`], as `chrono` panicked outside its range.
#[must_use]
pub fn days(n: i64) -> SignedDuration {
    match n.checked_mul(86_400) {
        Some(seconds) => SignedDuration::from_secs(seconds),
        None => panic!("{n} days do not fit a duration"),
    }
}

/// The whole days in `duration`, toward zero, as `chrono`'s `num_days` counted them.
#[must_use]
pub fn whole_days(duration: SignedDuration) -> i64 {
    duration.as_secs() / 86_400
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_rfc3339())
    }
}

impl FromStr for Timestamp {
    type Err = jiff::Error;

    /// What `chrono`'s `DateTime` read from text, and so from JSON: RFC 3339 with an offset (`Z` or
    /// `±hh:mm`), relaxed as [`chrono_text::timestamp`] says. Also what `jiff` reads, such as
    /// Postgres's text form (`2026-11-17 10:00:00+00`).
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match chrono_text::timestamp(text) {
            Some(instant) => Ok(Self(instant)),
            None => text.parse::<jiff::Timestamp>().map(Self),
        }
    }
}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_rfc3339_z())
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

impl schemars::JsonSchema for Timestamp {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DateTime".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string", "format": "date-time" })
    }

    fn inline_schema() -> bool {
        true
    }
}

/// A calendar day with no time or zone, as a `date` column holds one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date(pub civil::Date);

impl Date {
    /// The day `year`-`month`-`day`, or `None` for a day that does not exist.
    #[must_use]
    pub fn from_ymd(year: i16, month: i8, day: i8) -> Option<Self> {
        civil::Date::new(year, month, day).ok().map(Self)
    }

    /// A date as `chrono`'s `NaiveDate::parse_from_str(text, "%Y-%m-%d")` read one: as
    /// [`Date::from_str`], with no space before a `-` or after the day.
    #[must_use]
    pub fn parse_ymd(text: &str) -> Option<Self> {
        chrono_text::date_ymd(text).map(Self)
    }

    /// The instant at `time` on this day in UTC; `None` outside `jiff`'s range.
    #[must_use]
    pub fn at_utc(self, time: civil::Time) -> Option<Timestamp> {
        jiff::tz::TimeZone::UTC
            .to_timestamp(self.0.to_datetime(time))
            .ok()
            .map(Timestamp)
    }

    /// Today in UTC, on this process's clock.
    #[must_use]
    pub fn today_utc() -> Self {
        Timestamp::now().date_utc()
    }

    #[cfg(any(test, feature = "postgres"))]
    pub(crate) fn days_from_pg_epoch(self) -> Result<i32, String> {
        self.0
            .since((jiff::Unit::Day, PG_EPOCH_DATE))
            .map(|span| span.get_days())
            .map_err(|e| e.to_string())
    }

    #[cfg(any(test, feature = "postgres"))]
    pub(crate) fn from_pg_days(days: i32) -> Result<Self, String> {
        jiff::Span::new()
            .try_days(days)
            .and_then(|span| PG_EPOCH_DATE.checked_add(span))
            .map(Self)
            .map_err(|e| e.to_string())
    }
}

impl std::ops::Add<SignedDuration> for Date {
    type Output = Self;

    /// The day `by` later, in whole days of 24 hours (a part day is dropped, as `chrono` dropped
    /// it).
    ///
    /// # Panics
    ///
    /// When the result is outside `jiff`'s range.
    fn add(self, by: SignedDuration) -> Self {
        Self(self.0 + by)
    }
}

impl std::ops::Sub<SignedDuration> for Date {
    type Output = Self;

    /// The day `by` earlier, in whole days of 24 hours.
    ///
    /// # Panics
    ///
    /// When the result is outside `jiff`'s range.
    fn sub(self, by: SignedDuration) -> Self {
        Self(self.0 - by)
    }
}

impl std::ops::Sub for Date {
    type Output = SignedDuration;

    /// The days from `earlier` to this day, as so many 24 hours (negative when `earlier` is
    /// later), as `chrono` subtracted two dates.
    fn sub(self, earlier: Self) -> SignedDuration {
        self.0.duration_since(earlier.0)
    }
}

impl From<civil::Date> for Date {
    fn from(value: civil::Date) -> Self {
        Self(value)
    }
}

impl From<Date> for civil::Date {
    fn from(value: Date) -> Self {
        value.0
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl FromStr for Date {
    type Err = jiff::Error;

    /// What `chrono`'s `NaiveDate` read from text, and so from JSON: `YYYY-MM-DD`, relaxed as
    /// [`chrono_text::date`] says (`2026-2-8` is read). A time or a zone after it is refused, as
    /// `chrono` refused it.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        chrono_text::date(text).map(Self).ok_or_else(|| {
            jiff::Error::from_args(format_args!("a date is YYYY-MM-DD, not {text:?}"))
        })
    }
}

impl Serialize for Date {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

impl schemars::JsonSchema for Date {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Date".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string", "format": "date" })
    }

    fn inline_schema() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> Timestamp {
        text.parse().unwrap()
    }

    // The JSON a record carried under `chrono`, so no API response changes (CRM-T258).
    #[test]
    fn a_timestamp_serializes_as_chrono_wrote_it() {
        for (given, written) in [
            ("2026-11-17T10:00:00Z", "2026-11-17T10:00:00Z"),
            ("2026-11-17T10:00:00.5Z", "2026-11-17T10:00:00.500Z"),
            ("2026-11-17T10:00:00.1234Z", "2026-11-17T10:00:00.123400Z"),
            ("2026-11-17T10:00:00.123456Z", "2026-11-17T10:00:00.123456Z"),
            (
                "2026-11-17T10:00:00.000000001Z",
                "2026-11-17T10:00:00.000000001Z",
            ),
            ("2026-11-17T11:30:00+01:30", "2026-11-17T10:00:00Z"),
        ] {
            assert_eq!(serde_json::to_value(at(given)).unwrap(), written, "{given}");
            // `chrono`'s `to_rfc3339` wrote the same with `+00:00`.
            assert_eq!(
                at(given).to_rfc3339(),
                written.replace('Z', "+00:00"),
                "{given}"
            );
        }
    }

    #[test]
    fn a_timestamp_reads_what_a_client_or_postgres_sends() {
        let want = at("2026-11-17T10:00:00Z");
        for text in [
            "2026-11-17T10:00:00+00:00",
            "2026-11-17T12:00:00+02:00",
            "2026-11-17 10:00:00+00",
        ] {
            assert_eq!(
                serde_json::from_value::<Timestamp>(text.into()).unwrap(),
                want,
                "{text}"
            );
        }
        assert!(serde_json::from_value::<Timestamp>("2026-11-17T10:00:00".into()).is_err());
        assert!(serde_json::from_value::<Timestamp>("yesterday".into()).is_err());
    }

    #[test]
    fn a_timestamp_is_microseconds_from_2000_in_postgres() {
        assert_eq!(at("2000-01-01T00:00:00Z").to_pg_micros().unwrap(), 0);
        assert_eq!(
            at("2000-01-01T00:00:01.000002Z").to_pg_micros().unwrap(),
            1_000_002
        );
        assert_eq!(
            at("1999-12-31T23:59:59Z").to_pg_micros().unwrap(),
            -1_000_000
        );
        for micros in [0, 1, -1, 848_225_400_123_456, -946_684_800_000_000] {
            assert_eq!(
                Timestamp::from_pg_micros(micros)
                    .unwrap()
                    .to_pg_micros()
                    .unwrap(),
                micros
            );
        }
    }

    #[test]
    fn a_date_serializes_as_yyyy_mm_dd_and_reads_what_chrono_read() {
        let date = Date::from_ymd(2026, 2, 28).unwrap();
        assert_eq!(serde_json::to_value(date).unwrap(), "2026-02-28");
        // `chrono` read an unpadded field and spaces around the parts.
        for good in ["2026-02-28", "2026-2-28", " 2026 - 02 - 28 ", "+2026-02-28"] {
            assert_eq!(
                serde_json::from_value::<Date>(good.into()).unwrap(),
                date,
                "{good}"
            );
        }
        for bad in [
            "2026-02-30",
            "02026-02-28",
            "2026-02-28T00:00:00",
            "20260228",
            "28/02/2026",
        ] {
            assert!(serde_json::from_value::<Date>(bad.into()).is_err(), "{bad}");
        }
        assert_eq!(Date::from_ymd(2026, 2, 29), None);
        // `parse_from_str(text, "%Y-%m-%d")` took no space before a `-` or at the end.
        assert_eq!(Date::parse_ymd("2026-2-28"), Some(date));
        assert_eq!(Date::parse_ymd("2026 -02-28"), None);
        assert_eq!(Date::parse_ymd("2026-02-28 "), None);
    }

    #[test]
    fn a_timestamp_reads_what_chrono_read_and_rfc3339_strictly() {
        let want = at("2026-11-17T10:00:00Z");
        for text in [
            "2026-11-17t10:00:00z",
            " 2026-11-17T10:00:00 UTC ",
            "2026-11-17T10:00:00+0000",
            "2026-11-17T12:00:00 +02:00",
            "2026-11-17T10:0:0Z",
        ] {
            assert_eq!(text.parse::<Timestamp>().unwrap(), want, "{text}");
        }
        // More than nine digits of a fraction are cut to nine, as `chrono` cut them.
        assert_eq!(
            "2026-11-17T10:00:00.1234567891Z"
                .parse::<Timestamp>()
                .unwrap(),
            at("2026-11-17T10:00:00.123456789Z")
        );
        assert_eq!(
            Timestamp::parse_rfc3339("2026-11-17T12:00:00+02:00"),
            Some(want)
        );
        assert_eq!(Timestamp::parse_rfc3339("2026-11-17 10:00:00Z"), Some(want));
        for strict in [
            "2026-11-17T10:00:00+0000",
            "2026-11-17T10:00Z",
            "2026-11-17T10:00:00 Z",
            "2026-11-17T10:00:00+00",
        ] {
            assert_eq!(Timestamp::parse_rfc3339(strict), None, "{strict}");
        }
    }

    #[test]
    fn a_date_comes_back_from_its_days_from_2000() {
        for (days, ymd) in [
            (0, (2000, 1, 1)),
            (1, (2000, 1, 2)),
            (-1, (1999, 12, 31)),
            (9817, (2026, 11, 17)),
        ] {
            let date = Date::from_ymd(ymd.0, ymd.1, ymd.2).unwrap();
            assert_eq!(Date::from_pg_days(days).unwrap(), date, "{days}");
            assert_eq!(date.days_from_pg_epoch().unwrap(), days);
        }
        assert!(Date::from_pg_days(i32::MAX).is_err());
    }

    #[test]
    fn a_date_is_days_from_2000_in_postgres() {
        assert_eq!(
            Date::from_ymd(2000, 1, 1)
                .unwrap()
                .days_from_pg_epoch()
                .unwrap(),
            0
        );
        assert_eq!(
            Date::from_ymd(2000, 1, 2)
                .unwrap()
                .days_from_pg_epoch()
                .unwrap(),
            1
        );
        assert_eq!(
            Date::from_ymd(1999, 12, 31)
                .unwrap()
                .days_from_pg_epoch()
                .unwrap(),
            -1
        );
        assert_eq!(
            Date::from_ymd(2026, 11, 17)
                .unwrap()
                .days_from_pg_epoch()
                .unwrap(),
            9817
        );
    }

    #[test]
    fn the_utc_day_and_arithmetic() {
        let late = at("2026-11-17T23:59:59Z");
        assert_eq!(late.date_utc(), Date::from_ymd(2026, 11, 17).unwrap());
        assert_eq!(
            (late + SignedDuration::from_secs(1)).date_utc(),
            Date::from_ymd(2026, 11, 18).unwrap()
        );
        assert_eq!(
            late.duration_since(at("2026-11-17T23:59:00Z")),
            SignedDuration::from_secs(59)
        );
        assert_eq!(
            late - at("2026-11-17T23:59:00Z"),
            SignedDuration::from_secs(59)
        );
        assert_eq!(late - days(1), at("2026-11-16T23:59:59Z"));
    }

    #[test]
    fn strftime_writes_utc() {
        let now = at("2026-10-13T03:02:03+02:00");
        assert_eq!(
            now.strftime_utc("%Y%m%dT%H%M%SZ").to_string(),
            "20261013T010203Z"
        );
        assert_eq!(
            now.strftime_utc("%a, %d %b %Y %H:%M:%S GMT").to_string(),
            "Tue, 13 Oct 2026 01:02:03 GMT"
        );
    }

    #[test]
    fn a_day_and_a_clock_time_in_utc() {
        let day = Date::from_ymd(2026, 11, 17).unwrap();
        assert_eq!(
            day.at_utc(civil::Time::midnight()),
            Some(at("2026-11-17T00:00:00Z"))
        );
        assert_eq!(
            day.at_utc(civil::time(23, 59, 59, 0)),
            Some(at("2026-11-17T23:59:59Z"))
        );
        assert_eq!(at("2026-11-17T23:30:00+02:00").utc().hour(), 21);
    }

    #[test]
    fn a_date_moves_by_whole_days() {
        let day = Date::from_ymd(2026, 3, 1).unwrap();
        assert_eq!(day - days(1), Date::from_ymd(2026, 2, 28).unwrap());
        assert_eq!(day + days(365), Date::from_ymd(2027, 3, 1).unwrap());
        // A part day is dropped, toward zero.
        assert_eq!(
            day + SignedDuration::from_hours(47),
            Date::from_ymd(2026, 3, 2).unwrap()
        );
        assert_eq!(
            day - SignedDuration::from_hours(47),
            Date::from_ymd(2026, 2, 28).unwrap()
        );
        assert_eq!(whole_days(Date::from_ymd(2027, 3, 1).unwrap() - day), 365);
        assert_eq!(whole_days(Date::from_ymd(2026, 2, 1).unwrap() - day), -28);
        assert_eq!(whole_days(SignedDuration::from_hours(-47)), -1);
    }
}
