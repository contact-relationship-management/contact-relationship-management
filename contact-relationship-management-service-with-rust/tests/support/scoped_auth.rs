//! Settings changed for one test, put back when it ends.
//!
//! The token verifier and the ABAC policy are built from the process's settings, so a test that
//! signs in with its own keys and policy changes the settings, and puts them back afterwards,
//! including by panic. With that the sign-in tests share a process with the others.

use std::sync::Arc;

use contact_relationship_management::settings::{self, Settings};

/// Restores the settings it found, and rebuilds the verifier and policy from them.
pub struct Scoped(Arc<Settings>);

/// Change the process's settings with `edit` until the returned guard is dropped.
pub fn scoped(edit: impl FnOnce(&mut Settings)) -> Scoped {
    let before = settings::current();
    settings::update(edit);
    contact_relationship_management::auth::reload_from_settings();
    Scoped(before)
}

impl Drop for Scoped {
    fn drop(&mut self) {
        settings::install((*self.0).clone());
        contact_relationship_management::auth::reload_from_settings();
    }
}
