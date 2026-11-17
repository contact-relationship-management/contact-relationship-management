//! Environment variables set for one test, put back when it ends (CRM-T263).
//!
//! The app reads some settings from the environment each time it boots (the request limit, the
//! address lockout, compression, CORS, Keycloak). Since every integration test shares one process,
//! a test that sets one must put it back, including by panic, or the tests after it boot with it.
//! Only `#[serial]` tests set variables, so no other test reads the environment meanwhile.

/// The variables a test set, each with the value it found, restored in reverse order on drop.
#[derive(Default)]
pub struct ScopedEnv(Vec<(String, Option<String>)>);

impl ScopedEnv {
    /// Set `name` to `value` until this guard is dropped.
    pub fn set(&mut self, name: &str, value: &str) {
        if !self.0.iter().any(|(seen, _)| seen == name) {
            self.0.push((name.to_string(), std::env::var(name).ok()));
        }
        // Safe here: only `#[serial]` tests call this, so no other thread reads the environment.
        unsafe { std::env::set_var(name, value) };
    }
}

impl Drop for ScopedEnv {
    fn drop(&mut self) {
        for (name, before) in self.0.drain(..).rev() {
            // Safe as above.
            unsafe {
                match before {
                    Some(value) => std::env::set_var(&name, value),
                    None => std::env::remove_var(&name),
                }
            }
        }
    }
}
