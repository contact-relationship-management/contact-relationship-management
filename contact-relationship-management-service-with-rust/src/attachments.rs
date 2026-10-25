//! Where attached files live (CRM-R42): a directory on disk, and the scan hook run on each upload.
//!
//! A file is written under a name made of a random id, never of the name the person gave it, so a
//! name cannot reach outside the directory. It is written to `incoming/` first, scanned there when a
//! scan command is configured, and only then moved into place; one that is infected, or that could
//! not be scanned, is deleted and never becomes visible.

use std::path::{Path, PathBuf};
use std::time::Duration;

use uuid::Uuid;

/// How long a scan may take.
const SCAN_TIMEOUT: Duration = Duration::from_secs(60);

/// Why a file was not stored.
#[derive(Debug)]
pub enum StoreError {
    /// The scan found something.
    Infected,
    /// The scan could not run or did not finish; the file is refused rather than trusted.
    ScanFailed(String),
    /// The disk refused.
    Io(std::io::Error),
}

impl From<std::io::Error> for StoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

fn place(dir: &Path, key: Uuid) -> PathBuf {
    let name = key.to_string();
    dir.join(&name[..2]).join(name)
}

/// Run the scan command on `path`: exit 0 is clean, 1 is infected, anything else is a failure.
async fn scan(command: &str, path: &Path) -> Result<(), StoreError> {
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else {
        return Ok(());
    };
    let run = tokio::process::Command::new(program)
        .args(parts)
        .arg(path)
        .kill_on_drop(true)
        .output();
    match tokio::time::timeout(SCAN_TIMEOUT, run).await {
        Err(_) => Err(StoreError::ScanFailed("the scan timed out".to_string())),
        Ok(Err(error)) => Err(StoreError::ScanFailed(format!(
            "the scan could not start: {error}"
        ))),
        Ok(Ok(output)) => match output.status.code() {
            Some(0) => Ok(()),
            Some(1) => Err(StoreError::Infected),
            other => Err(StoreError::ScanFailed(format!(
                "the scan answered {}",
                other.map_or_else(|| "with a signal".to_string(), |c| c.to_string())
            ))),
        },
    }
}

/// Store `bytes` under `key`, scanning first when `scanner` is set.
///
/// # Errors
///
/// [`StoreError`]: nothing is left on disk in any of those cases.
pub async fn put(
    dir: &Path,
    key: Uuid,
    bytes: &[u8],
    scanner: Option<&str>,
) -> Result<(), StoreError> {
    let incoming = dir.join("incoming");
    tokio::fs::create_dir_all(&incoming).await?;
    let staged = incoming.join(key.to_string());
    tokio::fs::write(&staged, bytes).await?;
    if let Some(command) = scanner
        && let Err(error) = scan(command, &staged).await
    {
        let _ = tokio::fs::remove_file(&staged).await;
        return Err(error);
    }
    let target = place(dir, key);
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    if let Err(error) = tokio::fs::rename(&staged, &target).await {
        let _ = tokio::fs::remove_file(&staged).await;
        return Err(error.into());
    }
    Ok(())
}

/// Read a stored file.
///
/// # Errors
///
/// When the file is missing or unreadable.
pub async fn get(dir: &Path, key: Uuid) -> std::io::Result<Vec<u8>> {
    tokio::fs::read(place(dir, key)).await
}

/// Delete a stored file; a file already gone is not an error.
///
/// # Errors
///
/// When the disk refuses for any other reason.
pub async fn remove(dir: &Path, key: Uuid) -> std::io::Result<()> {
    match tokio::fs::remove_file(place(dir, key)).await {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}
