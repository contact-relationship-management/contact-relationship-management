//! `contact-relationship-management` binary entrypoint — parses the command line
//! and hands it to [`App::run`]. See the crate
//! library docs for the service overview.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use contact_relationship_management::app::App;

#[tokio::main]
async fn main() -> axa::Result<()> {
    App::run(axa::cli::parse(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION"),
    ))
    .await
}
