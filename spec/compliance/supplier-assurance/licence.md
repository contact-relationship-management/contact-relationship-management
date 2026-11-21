# Licence statement

*Written 2026-11-20 (CRM-T276, CRM-R110).* Not legal advice; not reviewed by a lawyer.

## The CRM's own licence

Every package in this repository (the service, the domain crate, the matcher and the UI) is offered under
`MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only OR OGL-UK-3.0 OR CC-BY-NC-SA-4.0`, at the user's choice ([LICENSE.md](../../../LICENSE.md)).

- **Hosting is allowed.** Each of the first six allows use and modification for any purpose, commercial hosting included. `CC-BY-NC-SA-4.0` allows non-commercial use only, so a commercial host chooses another option.
- **None asks a hosted service to publish its source.** No package is under the AGPL or another licence whose duties
  start when software is offered over a network. The GPL options attach duties to *distributing* copies, and a supplier
  that only hosts can choose MIT, Apache-2.0, BSD-3-Clause or OGL-UK-3.0 instead.
- **Configuration and data are never part of the licensed work.** A customer's settings, keys, records and templates are
  the customer's, whichever option is chosen.

## Dependencies

- **Rust:** `cargo deny check licenses` runs in the `supply-chain` stage for the service's workspace and for the matcher.
  It allows permissive licences and MPL-2.0 only ([deny.toml](../../../contact-relationship-management-service-with-rust/deny.toml));
  any other licence fails the build. MPL-2.0's duties are per file and start on distribution, not on hosting.
- **UI (npm):** **no licence check runs.** The UI's dependencies are pinned in `pnpm-lock.yaml`, and the production ones are
  listed in the CycloneDX SBOM (`pnpm sbom --prod`), but nothing fails on a licence. A supplier relying on this statement should check that SBOM itself.
- **Images:** the service and UI images are built on distroless base images. Their packages are not licence-checked here.

## Not done

A check of the UI's and the images' licences; a review by a lawyer.
