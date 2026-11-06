//! What the real binary says about itself, and what it refuses before it connects (CRM-T200).

use std::process::{Command, Output};

fn run(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_contact-relationship-management"));
    command.args(args).env_remove("DB_ALLOW_INSECURE");
    for (name, value) in env {
        command.env(name, value);
    }
    command.output().expect("the binary runs")
}

#[test]
fn version_names_this_service_and_not_axa() {
    let text = String::from_utf8(run(&["--version"], &[]).stdout).unwrap();
    assert!(text.starts_with(env!("CARGO_PKG_NAME")), "{text}");
    assert!(text.contains(env!("CARGO_PKG_VERSION")), "{text}");
}

#[test]
fn help_describes_this_service() {
    let text = String::from_utf8(run(&["--help"], &[]).stdout).unwrap();
    assert!(text.contains(env!("CARGO_PKG_DESCRIPTION")), "{text}");
}

#[test]
fn production_refuses_a_database_address_that_could_go_in_the_clear() {
    let production = [
        ("APP_ENV", "production"),
        ("DATABASE_URL", "postgres://crm:crm@db.example.com:5432/crm"),
        ("CRM_REDIS_URL", "redis://redis.example.com:6379"),
    ];
    let refused = run(&["check"], &production);
    let text = String::from_utf8_lossy(&refused.stderr);
    assert!(!refused.status.success());
    assert!(text.contains("sslmode"), "{text}");
    assert!(
        !text.contains("crm:crm"),
        "the password must not be printed: {text}"
    );
}
