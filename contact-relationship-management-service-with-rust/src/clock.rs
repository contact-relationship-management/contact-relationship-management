//! One clock for records (CRM-T221): the database's.
//!
//! The database stamps most rows itself (`created_at DEFAULT now()`), its own statements compare
//! with `now()`, and axa's queue runs jobs on its clock. The service used to stamp and compare
//! with the host's clock as well, so when the two disagreed a reminder fired late, a sweep and a
//! handler disagreed about a deadline, and a ticket opened before it was created. [`now`] is the
//! host's clock moved by the measured gap to the database's, so every replica follows the one
//! clock they share.
//!
//! The gap is measured at start-up and every [`SYNC_EVERY`], from the midpoint of a
//! `SELECT now()`, so it is off by at most half that round trip plus whatever the host drifts
//! between two measurements. A gap over [`WARN_AFTER`] is logged, since it means a host clock is
//! wrong.
//!
//! The host's clock stays where the other side of a check is someone else's clock: a signed
//! request's timestamp window (`webhook`, the WPM bridge, Mailgun) and token expiry, which the
//! verifying libraries read themselves.

use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

/// How often the gap is measured again.
pub const SYNC_EVERY: Duration = Duration::from_secs(60);

/// A gap larger than this is logged as a warning.
pub const WARN_AFTER: TimeDelta = TimeDelta::seconds(1);

/// How long start-up waits for the first measurement before going on with the last one.
const FIRST_SYNC_WAIT: Duration = Duration::from_secs(5);

/// The database's clock minus the host's, in milliseconds.
static OFFSET_MS: AtomicI64 = AtomicI64::new(0);

/// Whether this process measures already, so a server that is also the worker measures once.
static STARTED: AtomicBool = AtomicBool::new(false);

/// The time now, on the database's clock.
#[must_use]
pub fn now() -> DateTime<Utc> {
    shifted(Utc::now(), offset())
}

fn shifted(host: DateTime<Utc>, offset: TimeDelta) -> DateTime<Utc> {
    host + offset
}

/// The database's clock minus the host's, as last measured; zero before the first measurement.
#[must_use]
pub fn offset() -> TimeDelta {
    TimeDelta::milliseconds(OFFSET_MS.load(Ordering::Relaxed))
}

fn set_offset(offset: TimeDelta) {
    OFFSET_MS.store(offset.num_milliseconds(), Ordering::Relaxed);
}

/// The gap, from a database reading `database` taken between host readings `sent` and
/// `received`: the database read its clock, at best guess, halfway between.
#[must_use]
pub fn gap(sent: DateTime<Utc>, database: DateTime<Utc>, received: DateTime<Utc>) -> TimeDelta {
    let midpoint = sent + (received - sent) / 2;
    database - midpoint
}

/// Measure the gap once and use it from now on.
///
/// # Errors
///
/// When the database cannot be asked; the last gap stays.
pub async fn sync(pool: &PgPool) -> Result<TimeDelta, sqlx::Error> {
    let sent = Utc::now();
    let database: DateTime<Utc> = sqlx::query_scalar("SELECT now()").fetch_one(pool).await?;
    let received = Utc::now();
    let measured = gap(sent, database, received);
    set_offset(measured);
    Ok(measured)
}

fn report(measured: TimeDelta) {
    if measured.abs() > WARN_AFTER {
        tracing::warn!(
            offset_ms = measured.num_milliseconds(),
            "this host's clock is off from the database's; record times follow the database's"
        );
    }
}

/// Measure the gap before serving, waiting at most a few seconds, then again every
/// [`SYNC_EVERY`] in the background. A second call in one process does nothing.
pub async fn start(pool: PgPool) {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    match tokio::time::timeout(FIRST_SYNC_WAIT, sync(&pool)).await {
        Ok(Ok(measured)) => report(measured),
        Ok(Err(error)) => tracing::warn!(%error, "could not read the database's clock"),
        Err(_) => tracing::warn!("the database's clock did not answer in time"),
    }
    tokio::spawn(async move {
        let mut every = tokio::time::interval(SYNC_EVERY);
        every.tick().await;
        loop {
            every.tick().await;
            match sync(&pool).await {
                Ok(measured) => report(measured),
                Err(error) => tracing::warn!(%error, "could not read the database's clock"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn at(ms: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(1_800_000_000_000 + ms).unwrap()
    }

    #[test]
    fn the_gap_is_taken_from_the_middle_of_the_round_trip() {
        assert_eq!(gap(at(0), at(50), at(100)), TimeDelta::zero());
        assert_eq!(gap(at(0), at(539_050), at(100)), TimeDelta::seconds(539));
        assert_eq!(gap(at(0), at(-1_950), at(100)), TimeDelta::seconds(-2));
    }

    #[test]
    fn now_is_the_host_clock_moved_by_the_offset() {
        assert_eq!(shifted(at(1_000), TimeDelta::seconds(-539)), at(-538_000));
        assert_eq!(shifted(at(1_000), TimeDelta::zero()), at(1_000));
    }

    /// The files that may still read the host's clock, and why. Anything else that stamps or
    /// compares a record's time takes [`now`].
    const HOST_CLOCK: &[(&str, &str)] = &[
        ("src/clock.rs", "the clock itself"),
        (
            "src/webhook.rs",
            "a provider's signed timestamp is on the provider's clock",
        ),
        (
            "src/controllers/bridges.rs",
            "WPM's signed timestamp is on WPM's clock",
        ),
        (
            "src/controllers/marketing/public.rs",
            "Mailgun's signed timestamp is on Mailgun's clock",
        ),
    ];

    fn rust_files(dir: &Path, found: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_files(&path, found);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }

    #[test]
    fn only_the_listed_files_read_the_host_clock() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        rust_files(&root.join("src"), &mut files);
        let needle = ["Utc", "::now()"].concat();
        let mut readers: Vec<String> = files
            .iter()
            .filter(|path| std::fs::read_to_string(path).unwrap().contains(&needle))
            .map(|path| path.strip_prefix(root).unwrap().display().to_string())
            .collect();
        readers.sort();
        let mut listed: Vec<&str> = HOST_CLOCK.iter().map(|(path, _)| *path).collect();
        listed.sort_unstable();
        assert_eq!(readers, listed);
    }
}
