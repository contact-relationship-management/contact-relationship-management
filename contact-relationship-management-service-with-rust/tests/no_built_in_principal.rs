//! CRM-R100: no built-in or vendor account exists in any build. Nothing the service ships may create
//! a person who can sign in: not a migration, not the image. Sign-in belongs to the customer's
//! identity provider, so a principal is never created here. The one password in the repository is the
//! development mail server's own account in `dev/stalwart/plan.ndjson`, which is a test fixture for
//! outgoing mail, not a CRM sign-in; the test below names it so a second one cannot slip in.

use std::fs;
use std::path::Path;

/// Every file in `dir` whose name ends in `suffix`, with its text.
fn texts(dir: &str, suffix: &str) -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let mut found: Vec<_> = fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("{}: {e}", root.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().ends_with(suffix))
        .map(|path| {
            (
                path.display().to_string(),
                fs::read_to_string(&path).unwrap(),
            )
        })
        .collect();
    found.sort();
    found
}

#[test]
fn no_migration_inserts_a_row_or_sets_a_password() {
    let migrations = texts("migrations", ".sql");
    assert!(
        !migrations.is_empty(),
        "no migrations found, so the check would pass for nothing"
    );
    for (path, sql) in migrations {
        let lower = sql.to_lowercase();
        assert!(
            !lower.contains("insert into"),
            "{path} inserts a row; a migration creates tables, never a principal"
        );
        assert!(
            !lower.contains("password"),
            "{path} mentions a password; the service holds none"
        );
    }
}

#[test]
fn the_image_sets_no_account_or_secret() {
    let image =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Containerfile")).unwrap();
    for line in image
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("ENV ") || l.starts_with("ARG "))
    {
        let upper = line.to_uppercase();
        for word in ["PASSWORD", "SECRET", "TOKEN", "ADMIN"] {
            assert!(
                !upper.contains(word),
                "the image sets `{line}`; a secret or account comes from the deployer"
            );
        }
    }
}

#[test]
fn the_only_seeded_password_is_the_development_mail_servers() {
    let plans = texts("dev/stalwart", ".ndjson");
    let with_password: Vec<_> = plans
        .iter()
        .filter(|(_, text)| text.contains("\"Password\""))
        .map(|(path, _)| path.as_str())
        .collect();
    assert_eq!(
        with_password.len(),
        1,
        "a seeded password beyond the dev mail server's: {with_password:?}"
    );
    assert!(with_password[0].ends_with("plan.ndjson"));
}
