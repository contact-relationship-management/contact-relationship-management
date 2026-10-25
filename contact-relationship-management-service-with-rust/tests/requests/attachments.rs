//! File attachments (CRM-R42): the allow-list, the scan hook, downloads that are always
//! attachments, the integrity check, subject access and erasure.

use std::path::{Path, PathBuf};

use axum::body::Bytes;
use contact_relationship_management::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::seed_contact;
use crate::scoped_auth::scoped;

/// A fresh directory for one test's files, removed when the test ends.
struct Dir(PathBuf);

impl Dir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("crm-attach-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn files(&self) -> usize {
        fn walk(p: &Path) -> usize {
            std::fs::read_dir(p)
                .map(|entries| {
                    entries
                        .flatten()
                        .map(|e| {
                            if e.path().is_dir() {
                                walk(&e.path())
                            } else {
                                1
                            }
                        })
                        .sum()
                })
                .unwrap_or(0)
        }
        walk(&self.0)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const PDF: &[u8] = b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\n%%EOF\n";

macro_rules! upload {
    ($request:expr, $contact:expr, $name:expr, $body:expr) => {
        $request
            .post(&format!(
                "/api/contacts/{}/attachments?name={}",
                $contact, $name
            ))
            .content_type("application/octet-stream")
            .bytes(Bytes::from($body))
            .await
    };
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_file_is_stored_listed_downloaded_and_deleted() {
    let dir = Dir::new();
    let path = dir.0.to_string_lossy().to_string();
    let _scope = scoped(|s| {
        s.attachment_dir = Some(path);
        s.attachment_max_bytes = 1000;
    });
    request::<App, _, _>(|request, _ctx| async move {
        let contact = seed_contact!(&request, "File Holder").await;
        let made = upload!(&request, contact, "..%2F..%2Fcontract.pdf", PDF.to_vec());
        made.assert_status_ok();
        let made: Value = made.json();
        assert_eq!(made["name"], "contract.pdf", "the path is dropped");
        assert_eq!(made["content_type"], "application/pdf");
        assert_eq!(made["size_bytes"], PDF.len());
        let pid = made["pid"].as_str().unwrap().to_string();
        assert_eq!(dir.files(), 1);

        let listed: Vec<Value> = request
            .get(&format!("/api/contacts/{contact}/attachments"))
            .await
            .json();
        assert_eq!(listed.len(), 1);

        let download = request
            .get(&format!("/api/attachments/{pid}/content"))
            .await;
        download.assert_status_ok();
        assert_eq!(download.as_bytes().as_ref(), PDF);
        let headers = download.headers();
        assert!(
            headers["content-disposition"]
                .to_str()
                .unwrap()
                .starts_with("attachment; filename=\"contract.pdf\"")
        );
        assert_eq!(headers["x-content-type-options"], "nosniff");
        assert_eq!(headers["content-type"], "application/pdf");

        // Refusals: a type off the list, a file that is not what its name says, an empty file,
        // one over the limit, an unknown record.
        for (name, body) in [
            ("run.exe", b"MZ".to_vec()),
            ("page.html", b"<script>alert(1)</script>".to_vec()),
            ("fake.pdf", b"<script>alert(1)</script>".to_vec()),
            ("empty.txt", Vec::new()),
            ("big.txt", vec![b'x'; 1001]),
        ] {
            let r = upload!(&request, contact, name, body);
            r.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        }
        upload!(&request, uuid::Uuid::new_v4(), "a.txt", b"hi".to_vec())
            .assert_status(axum::http::StatusCode::NOT_FOUND);
        assert_eq!(dir.files(), 1, "nothing refused was kept");

        // A file changed on disk is not served.
        let stored = walkdir_first(&dir.0);
        std::fs::write(&stored, b"%PDF-tampered").unwrap();
        request
            .get(&format!("/api/attachments/{pid}/content"))
            .await
            .assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);

        // Deleting removes the row and the file.
        request
            .delete(&format!("/api/attachments/{pid}"))
            .await
            .assert_status_ok();
        assert_eq!(dir.files(), 0);
        request
            .get(&format!("/api/attachments/{pid}/content"))
            .await
            .assert_status(axum::http::StatusCode::NOT_FOUND);
    })
    .await;
}

fn walkdir_first(root: &Path) -> PathBuf {
    for entry in std::fs::read_dir(root).unwrap().flatten() {
        let p = entry.path();
        if p.is_dir() {
            if let Some(found) = std::fs::read_dir(&p)
                .unwrap()
                .flatten()
                .map(|e| e.path())
                .find(|q| q.is_file())
            {
                return found;
            }
            let inner = walkdir_first(&p);
            if inner.is_file() {
                return inner;
            }
        }
    }
    root.to_path_buf()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn attachments_are_off_until_a_directory_is_configured() {
    let _scope = scoped(|s| s.attachment_dir = None);
    request::<App, _, _>(|request, _ctx| async move {
        let contact = seed_contact!(&request, "No Disk").await;
        upload!(&request, contact, "a.txt", b"hi".to_vec())
            .assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_scan_hook_refuses_the_infected_and_the_unscannable() {
    let dir = Dir::new();
    let path = dir.0.to_string_lossy().to_string();
    let script = dir.0.join("scan.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\ncase \"$(cat \"$1\")\" in\n  *EICAR*) exit 1 ;;\n  *BREAK*) exit 2 ;;\nesac\nexit 0\n",
    )
    .unwrap();
    let command = format!("sh {}", script.display());
    let _scope = scoped(|s| {
        s.attachment_dir = Some(path);
        s.attachment_scan_command = Some(command);
    });
    request::<App, _, _>(|request, ctx| async move {
        use sea_orm::ConnectionTrait;
        let contact = seed_contact!(&request, "Scanned").await;
        let before = dir.files();
        upload!(&request, contact, "clean.txt", b"just words".to_vec()).assert_status_ok();
        assert_eq!(dir.files(), before + 1);
        let infected = upload!(&request, contact, "bad.txt", b"X5O EICAR test".to_vec());
        infected.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        let broken = upload!(&request, contact, "odd.txt", b"BREAK the scanner".to_vec());
        broken.assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(dir.files(), before + 1, "neither refused file was kept");
        let held = ctx
            .db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT count(*) AS n FROM attachments".to_string(),
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<i64>("", "n")
            .unwrap();
        assert_eq!(held, 1);
        let audited = ctx
            .db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT count(*) AS n FROM audit_logs WHERE entity = 'attachment' \
                 AND action = 'refused_infected'"
                    .to_string(),
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<i64>("", "n")
            .unwrap();
        assert_eq!(audited, 1);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn subject_access_lists_the_files_and_erasure_deletes_them() {
    let dir = Dir::new();
    let path = dir.0.to_string_lossy().to_string();
    let _scope = scoped(|s| s.attachment_dir = Some(path));
    request::<App, _, _>(|request, _ctx| async move {
        let contact = seed_contact!(&request, "Erase My Files").await;
        upload!(&request, contact, "a.txt", b"first".to_vec()).assert_status_ok();
        upload!(&request, contact, "b.pdf", PDF.to_vec()).assert_status_ok();
        assert_eq!(dir.files(), 2);
        let export: Value = request
            .get(&format!("/api/contacts/{contact}/subject-access"))
            .await
            .json();
        let names: Vec<&str> = export["attachments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["a.txt", "b.pdf"]);
        request
            .post(&format!("/api/contacts/{contact}/erase"))
            .await
            .assert_status_ok();
        assert_eq!(dir.files(), 0, "the files are gone from disk");
        let _ = json!(null);
    })
    .await;
}
