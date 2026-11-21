//! What the real binary says about itself, and what it refuses before it connects (CRM-T200,
//! CRM-T201).

// They start the binary, which reads the environment, so they wait for any test that sets it
// (CRM-T263).
use serial_test::serial;

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
#[serial]
fn version_names_this_service_and_not_axa() {
    let text = String::from_utf8(run(&["--version"], &[]).stdout).unwrap();
    assert!(text.starts_with(env!("CARGO_PKG_NAME")), "{text}");
    assert!(text.contains(env!("CARGO_PKG_VERSION")), "{text}");
}

#[test]
#[serial]
fn help_describes_this_service() {
    let text = String::from_utf8(run(&["--help"], &[]).stdout).unwrap();
    assert!(text.contains(env!("CARGO_PKG_DESCRIPTION")), "{text}");
}

#[test]
#[serial]
fn production_refuses_a_database_address_that_could_go_in_the_clear() {
    let production = [
        ("APP_ENV", "production"),
        ("DB_URL", "postgres://crm:crm@db.example.com:5432/crm"),
        (
            "CONTACT_RELATIONSHIP_MANAGEMENT_REDIS_URL",
            "redis://redis.example.com:6379",
        ),
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
#[serial]
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
#[serial]
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

// The commands below connect, so they need a PostgreSQL database named by `DB_URL`
// (`cargo test -- --ignored`, as the other database tests do). They run the binary the way an
// operator does, so `App::run` and `App::db` are exercised end to end (CRM-T292, round 11).

/// The binary in development against the test database, with the guard on destructive commands open.
fn operate(args: &[&str]) -> Output {
    let url = std::env::var("DB_URL").expect("DB_URL names a PostgreSQL database");
    run(
        args,
        &[
            ("APP_ENV", "development"),
            ("AXA_DESTROY_DATA", "true"),
            ("DB_URL", &url),
        ],
    )
}

fn out(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The migration lines of `db status`, without the log lines the binary also prints.
fn migration_lines(output: &Output) -> Vec<String> {
    out(output)
        .lines()
        .filter(|line| line.starts_with("applied") || line.starts_with("pending"))
        .map(str::to_string)
        .collect()
}

fn err(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
fn db_status_names_the_baseline_after_a_migrate() {
    let migrated = operate(&["db", "migrate"]);
    assert!(migrated.status.success(), "{}", err(&migrated));
    let status = operate(&["db", "status"]);
    assert!(status.status.success(), "{}", err(&status));
    let lines = migration_lines(&status);
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("applied") && l.contains("baseline")),
        "{lines:?}"
    );
}

#[test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
fn db_down_says_there_is_no_down_migration_and_changes_nothing() {
    let before = migration_lines(&operate(&["db", "status"]));
    assert!(!before.is_empty());
    let refused = operate(&["db", "down", "1"]);
    assert!(!refused.status.success());
    let text = err(&refused);
    assert!(text.contains("no down migration"), "{text}");
    assert_eq!(migration_lines(&operate(&["db", "status"])), before);
}

#[test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
fn db_reset_and_truncate_leave_a_migrated_database() {
    for command in ["reset", "truncate"] {
        let done = operate(&["db", command]);
        assert!(done.status.success(), "db {command}: {}", err(&done));
    }
    let lines = migration_lines(&operate(&["db", "status"]));
    assert!(lines.iter().any(|l| l.contains("baseline")), "{lines:?}");
}

#[test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test -- --ignored`"]
fn task_lists_what_can_be_run_and_refuses_one_it_does_not_have() {
    let listed = operate(&["task"]);
    assert!(listed.status.success(), "{}", err(&listed));
    let text = out(&listed);
    for name in ["seed", "rotate_field_keys"] {
        assert!(text.contains(name), "{name} is not listed: {text}");
    }
    let unknown = operate(&["task", "no_such_task"]);
    assert!(!unknown.status.success());
}
