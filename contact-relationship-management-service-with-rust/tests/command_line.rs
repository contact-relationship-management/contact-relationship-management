//! What the real binary says about itself, and what it refuses before it connects (CRM-T200,
//! CRM-T201).

use std::process::{Command, Output};

fn run(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_contact-relationship-management"));
    command
        .args(args)
        .env_remove("DB_ALLOW_INSECURE")
        .env_remove("AXA_DESTROY_DATA");
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
        ("DB_URL", "postgres://crm:crm@db.example.com:5432/crm"),
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

/// No database is reachable here, so a refusal that names the setting can only come from the
/// check that runs before the service connects.
#[test]
fn commands_that_destroy_data_are_refused_before_connecting() {
    let unreachable = "postgres://crm:crm@127.0.0.1:1/crm";
    for command in [
        ["db", "down", "1"].as_slice(),
        ["db", "reset"].as_slice(),
        ["db", "truncate"].as_slice(),
    ] {
        for env in [
            [("APP_ENV", "development"), ("DB_URL", unreachable)].as_slice(),
            [
                ("APP_ENV", "production"),
                ("AXA_DESTROY_DATA", "true"),
                ("DB_URL", unreachable),
            ]
            .as_slice(),
        ] {
            let refused = run(command, env);
            let text = String::from_utf8_lossy(&refused.stderr);
            assert!(!refused.status.success(), "{command:?} {env:?}");
            assert!(
                text.contains("AXA_DESTROY_DATA"),
                "{command:?} {env:?}: {text}"
            );
        }
    }
}

/// The setting opens the guard outside production: this call gets as far as the database.
#[test]
fn the_setting_lets_a_destructive_command_through_in_development() {
    let attempted = run(
        &["db", "down", "1"],
        &[
            ("APP_ENV", "development"),
            ("AXA_DESTROY_DATA", "true"),
            ("DB_URL", "postgres://crm:crm@127.0.0.1:1/crm"),
            ("DB_CONNECT_TIMEOUT", "100"),
        ],
    );
    let text = String::from_utf8_lossy(&attempted.stderr);
    assert!(!attempted.status.success());
    assert!(!text.contains("AXA_DESTROY_DATA"), "{text}");
}
