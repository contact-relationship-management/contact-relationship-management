//! The access policy from settings (CRM-T292): an inline policy, a policy file, and the built-in
//! default when the file cannot be read or does not parse; a reload swapping the live policy; the
//! watcher reloading when the file changes. Serial, as the settings and the live policy are
//! process-wide; no database.

use authentication_verifier::Policy;
use contact_relationship_management::auth::{
    policy, policy_from_env, reload_policy, spawn_policy_watcher,
};
use contact_relationship_management::settings::{self, Settings};
use serial_test::serial;

const CUSTOM: &str = r#"{ "rules": [
    { "effect": "allow", "actions": ["read"], "when": { "access": ["auditor"] } }
] }"#;

/// Puts the default settings and the default live policy back, even by panic.
struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        settings::install(Settings::default());
        reload_policy();
    }
}

/// A file of `contents` in a directory of its own under the system's temporary directory.
fn file(contents: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("crm-policy-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("policy.json");
    std::fs::write(&path, contents).unwrap();
    path
}

fn use_file(path: Option<&std::path::Path>, inline: Option<&str>) {
    settings::update(|s| {
        s.abac_policy_file = path.map(|p| p.to_string_lossy().into_owned());
        s.abac_policy = inline.map(str::to_string);
    });
}

#[test]
#[serial]
fn the_policy_comes_from_the_file_or_inline_and_falls_back_to_the_default() {
    let _restore = Restore;
    let custom = Policy::from_json(CUSTOM).unwrap();
    assert_ne!(custom, Policy::default_policy());

    use_file(Some(&file(CUSTOM)), None);
    assert_eq!(policy_from_env(), custom, "read from the file");

    use_file(Some(&file("{ not json")), None);
    assert_eq!(
        policy_from_env(),
        Policy::default_policy(),
        "a file that does not parse"
    );

    use_file(
        Some(std::path::Path::new("/nonexistent/crm/policy.json")),
        None,
    );
    assert_eq!(
        policy_from_env(),
        Policy::default_policy(),
        "a file that cannot be read"
    );

    use_file(Some(&file("{ not json")), Some(CUSTOM));
    assert_eq!(
        policy_from_env(),
        custom,
        "an inline policy wins over the file"
    );

    use_file(None, None);
    assert_eq!(policy_from_env(), Policy::default_policy());
}

#[test]
#[serial]
fn a_reload_swaps_the_live_policy() {
    let _restore = Restore;
    use_file(Some(&file(CUSTOM)), None);
    reload_policy();
    assert_eq!(*policy().current(), Policy::from_json(CUSTOM).unwrap());
    use_file(None, None);
    reload_policy();
    assert_eq!(*policy().current(), Policy::default_policy());
}

#[tokio::test(start_paused = true)]
#[serial]
async fn the_watcher_reloads_the_policy_when_its_file_changes() {
    let _restore = Restore;
    // No file: nothing to watch.
    use_file(None, None);
    spawn_policy_watcher();

    let path = file("{ \"rules\": [] }");
    use_file(Some(&path), None);
    reload_policy();
    assert_eq!(
        *policy().current(),
        Policy::from_json("{ \"rules\": [] }").unwrap()
    );
    spawn_policy_watcher();
    // Let the watcher read the file's time before it changes.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    // A later modification time, as an editor's save gives.
    std::fs::write(&path, CUSTOM).unwrap();
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(later)
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(16)).await;
    for _ in 0..50 {
        if *policy().current() == Policy::from_json(CUSTOM).unwrap() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(
        *policy().current(),
        Policy::from_json(CUSTOM).unwrap(),
        "reloaded from the changed file"
    );
}
