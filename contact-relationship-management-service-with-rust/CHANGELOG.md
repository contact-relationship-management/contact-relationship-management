# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — converted campaign ROI (CRM-T116, CRM-R52)

`GET /api/campaigns/{pid}/funnel` takes `convert` and `rate_date`; the answer names `rates` and `unconverted`.

### Added — converted attribution (CRM-T115, CRM-R51)

`GET /api/attribution` takes `convert_to` and `rate_date`; the answer names `rates` and `unconverted`. The conversion mapping now lives in `controllers::fx_rates`, shared with reports.

### Added — exchange rates (CRM-T114, CRM-R50)

`fx_rates` (migration `000038`), `GET/PUT /api/fx-rates` and `DELETE /api/fx-rates/{pid}`, audited. `rules::fx` reads rates as integers, picks the latest in force on a day (a pair's own, or the reverse inverted), and converts half to even. A report spec may carry `convert_to` and `rate_date`; the answer names `rates` and `unconverted`.

### Added — multi-touch attribution (CRM-T113, CRM-R49)

`GET /api/attribution?model=&window_days=&from=&to=`: each won deal's amount shared among the campaigns that sent its primary contact an e-mail, brought in a lead of theirs, or are the recorded source. `rules::attribution` does the sharing (shares add up to the amount; leftover minor units to the earliest touches); credit is per campaign and per currency, untouched deals are listed apart, a masked read gets `403`, and asking audits `attribution_run`.

### Changed — dependencies

`Cargo.lock` updated (`tokio` 1.54.1, `redis` 1.8, `uuid` 1.29 and others); `jsonschema` 0.58 to 0.60. Clippy (both feature sets) and the full request suite pass.

### Added — report builder (CRM-T112, CRM-R48)

`reports` (migration `000037`); `POST /api/reports/run` and saved reports under `/api/reports`. `rules::reports` plans the query from a closed vocabulary
with every value bound. A reader may run and save reports (they derive `read`).

### Added — tags (CRM-T111, CRM-R47)

`tags` and `taggings` (migration `000036`); `/api/tags`, `/api/{contacts,accounts,leads,deals}/{pid}/tags`, and `?tag=` on the four lists. A segment filter has `tags`
(any of), so `segment::Filter` and `ContactFacts` gained a field. The subject-access export has `tags`.

### Added — SLA working hours and holidays (CRM-T109, CRM-R45)

`business_calendars` and `sla_policies.calendar_pid` (migration `000035`); `/api/business-calendars` and `PUT /api/sla-policies/{pid}/calendar`.
`rules::calendar` counts working minutes; `sla::deadlines_on` uses it. New direct dependency `chrono-tz` 0.10 (already in the tree through `loco-rs`).

### Added — lead assignment (CRM-T108, CRM-R44)

`leads.owner_ref` and `workflows.assign_cursor` (migration `000034`). Trigger `lead_created`, condition `sources`, action `assign_owner`
(`round_robin` or `least_open`, optional notification). `DryRunSample` now names its subject: `subject_kind` and `subject_pid` replace `contact_pid`.

### Added — contact and account import (CRM-T107, CRM-R43)

`record_imports` (migration `000033`); `/api/contact-imports` and `/api/account-imports` (preview, commit, list, read, undo). `clients::resolve_organization` is the
organization-service counterpart of `resolve_person`. `lead_imports::advise` is now shared.

### Added — file attachments (CRM-T106, CRM-R42)

`attachments` (migration `000032`); `/api/{contacts,accounts,leads,deals}/{pid}/attachments`, `/api/attachments/{pid}/content` and `DELETE /api/attachments/{pid}`.
New settings `CRM_ATTACHMENT_DIR` (unset: attachments answer 503), `CRM_ATTACHMENT_MAX_BYTES` and `CRM_ATTACHMENT_SCAN_COMMAND`. The `tokio` features `fs`, `process` and `time` are now enabled.

### Added — typed custom fields (CRM-T105, CRM-R41)

`custom_field_defs` and `custom_field_values` (migration `000031`); `/api/custom-fields` and `/api/{contacts,accounts,leads,deals}/{pid}/custom-fields`.
Kinds: text, number, date, choice. Sensitivity `business`, `personal` or `sensitive` decides masking, sealing, the subject-access export and erasure.
`rotate_field_keys` re-seals the sensitive values.

### Added — explainable workflow rules (CRM-T104, CRM-R40)

`workflows` and `workflow_runs` (migration `000030`); `/api/workflows` (CRUD, `dry-run`, `runs`, `run-due`). Triggers: contact created or updated, a deal's
stage move, a schedule. Actions: `create_task`, `notify`, `set_field`, `enrol_nurture`, `send_webhook`. A failing action undoes the run; depth and rate
guards cut loops; `enrol_nurture` re-reads consent. Notifications have the new kind `workflow`.

### Added — editing an account (CRM-R39)

`PUT /api/accounts/{pid}` changes `display_name`, `tier`, `owner_ref` and `industry` (only the members sent; `null` clears). The audit row lists
the field names, never the values.

### Added — a contact's given name, and editing a contact (CRM-R38)

`contacts.given_name` (optional, migration `000029`), set on `POST /api/contacts`, filled by a merge, cleared by an unmerge and erased with the
contact. The `given_name` merge token uses it, else the first word of the display name. New `PUT /api/contacts/{pid}` changes the contact's own
fields (only the members sent; `null` clears); the audit row lists the field names, never the values.

### Added — a contact's family name (CRM-R37)

`contacts.family_name` (optional, migration `000028`), set on `POST /api/contacts`, filled by a merge and cleared by an unmerge, erased with the
contact. The new merge token `family_name` fills from it; it is never guessed from the display name.

### Added — email templates with merge tokens (CRM-T103, CRM-R36)

`/api/email-templates` stores templates (name, subject, plain-text body, fallbacks). Merge tokens come from a closed set (`given_name`, `name`,
`account_name`, `rep_name`, `job_title`, `unsubscribe_url`); each but the link has a declared fallback, which a template may override. Values are
HTML-escaped in the HTML form and cleaned of control characters, so a value cannot add a header line. Every body must contain
`{{unsubscribe_url}}` or the save is refused. `POST /api/email-templates/preview` renders a draft for a synthetic contact and lists what would
stop it being saved; a reader may use it. `POST /api/campaigns` and a nurture step accept `template_pid` in place of a subject and body; the
template is read when each message is made. A template in use cannot be deleted (409). Migration `000027`.

### Added — saved views (CRM-T102, CRM-R35)

`/api/views` stores per-user and shared views (a filter, a sort and columns) of the contacts, accounts, deals, leads and tickets lists. A filter
is checked against a closed set of fields and values per list. A view holds no record data and sharing one shares no access: the list it is
applied to is still the caller's own read. Making, changing and deleting your own views is open to readers. Migration `000026`.

### Added — CSV import of leads (CRM-T101, CRM-R34)

`POST /api/lead-imports/preview` reads a CSV (sent as text, up to 500 rows) and reports each row as valid or invalid with the reason, with
likely duplicates from the matcher as advice only. `POST /api/lead-imports` saves the valid rows as `source=import` leads in batches, recording
where the list came from and when it was obtained; no consent is inferred and only the address's domain is kept.
`POST /api/lead-imports/{pid}/undo` removes the leads still new and untouched. New dependency: `csv`. Migration `000025`.

### Added — server-side search (CRM-T100, CRM-R33)

`GET /api/search?q=` finds contacts, accounts, leads, deals, tickets, campaigns and articles by name or title, ignoring accents and
forgiving typos (PostgreSQL full text plus `pg_trgm`), best match first and paged. Records the caller may not read are left out and the
audit row records a count, not the text. Migration `000024` enables the `unaccent` and `pg_trgm` extensions and adds GIN indexes. The UI
search page now uses it.

### Added — tasks with reminders and a notifications inbox (CRM-T99, CRM-R32)

Task activities take an `assignee_ref` and a `remind_at`. A job the server queues every minute (and `POST /api/notifications/sweep`)
makes a notification for each due reminder and each SLA breach on an assigned ticket, once each. A notification is made only if the
assignee may read the record, and is checked again with the caller's own token each time the inbox is read. New routes:
`GET /api/notifications` (`?unread=true`), `GET /api/notifications/unread-count`, `PUT /api/notifications/{pid}/read`,
`POST /api/notifications/read-all`. New tables/columns in migration `000023`. Notifications carry no record text and are deleted on erasure.

### Added — web form lead capture (CRM-T98, CRM-R31)

`POST /forms/{slug}` (public, outside `/api`) turns a website sign-up form into a contact, a `source=web` lead and a `pending`
consent purpose, and mails one confirmation to the visitor; nothing else is sent until they press the button, and a nurture
sequence the form names starts then. Forms are configured in `CRM_WEB_FORMS` (JSON: `slug`, `origins`, optional `purpose` and
`sequence`). The request's `Origin` must be on the form's list; each form allows `CRM_WEB_FORM_CLIENT_LIMIT` (5) submissions per
client per 10 minutes and `CRM_WEB_FORM_LIMIT` (300) per hour, answering `429`; a filled `website` honeypot creates nothing; every
non-error answer is the same `202`. The address goes to the person service (`POST {CRM_PERSON_SERVICE_URL}/api/persons`) and is not
stored. New table `form_submissions` (migration `000022`), deleted on erasure. Counts are per process.

### Added — an MCP endpoint for AI agents (CRM-T97, CRM-R30)

`POST /mcp` speaks the Model Context Protocol over Streamable HTTP, statelessly: revision 2026-07-28, and 2025-11-25 and
2025-06-18 for clients that open with `initialize`. Tools only. Each tool is one REST operation sent through the service's own
router with the caller's own bearer token, so sign-in, the access policy, masking, rate limits and audit decide it as they decide
the same request from any client. The tools are read-only lookups; a merge or an undo is offered only as a preview that changes
nothing; `add_activity` appears only with `CRM_MCP_WRITES=on`; nothing erases. Every call is audited first with an `agent`
marker (and not refused only if the audit row can be written), results are marked as data, and browser origins must be listed in
`CRM_MCP_ALLOWED_ORIGINS`. Behaviour change in code, not in the API: `tower` is now a regular dependency.

### Added — consent per purpose and double opt-in (CRM-T96, CRM-R29)

Consent is held per purpose: `marketing` (the original, still `contacts.marketing_consent`), `newsletter`, `events` and
`product_updates`, each `granted`, `withdrawn`, `never` or the new `pending`. Only `granted` allows a send, and a
campaign or nurture sequence now names the purpose it is sent under (default `marketing`; set `purpose` when creating
it). `POST /api/contacts/{pid}/consent` takes an optional `purpose`; `GET /api/contacts/{pid}/consents` lists them all.
`POST /api/contacts/{pid}/consent/request` starts a double opt-in and returns a signed link (7 days) whose page confirms on
a button press, never on a plain visit, and never after a withdrawal. `POST /api/contacts/{pid}/preferences-link` returns a
signed page (90 days) where the person starts or stops each purpose. An unsubscribe link withdraws the purpose of its own
message only; a spam complaint withdraws all. Merge, unmerge and erasure apply per purpose. Existing data is the default
purpose, unchanged. Needs migration 000021 (`scripts/migrate-consent-purposes.sh` checks it against populated data).

### Added — outbound webhooks (CRM-T95, CRM-R28)

Admin and service callers register an `https` address and the events it wants (`contact.created`, `deal.*`, `*`) under
`/api/webhook-subscriptions`. `POST /api/webhook-subscriptions/relay` (call it on a schedule) turns new `event_outbox`
rows into deliveries and sends what is due. Deliveries are signed in the Standard Webhooks format (`webhook-id`,
`webhook-timestamp`, `webhook-signature`; two signatures for 24 hours after a secret rotates), carry ids and never contact
values, are retried with growing delays and then dead-lettered, and a `410 Gone` disables the subscription. Addresses
inside a network (private, loopback, link-local, metadata) are refused at registration and again, by resolved address, at
every send. Erasing a contact cancels its queued deliveries. At-least-once: receivers dedupe on `webhook-id`. Needs
migration 000020. New dependency feature: tokio `net`.

### Changed — response members are required in the OpenAPI document (CRM-T93)

Responses are described as the service writes them: an optional member is always present, as `null`. Before, it was
marked possibly absent, which made generated clients treat `null` and missing alike. Requests are unchanged (an
optional member may be left out). `cargo run --example openapi` prints the document.

### Changed — the OpenAPI document is generated from the code (CRM-T83)

- Every schema in `/api-docs/openapi.json` except `Error` now comes from the Rust type that is sent or received (the
  request payloads, the database models, and typed response structs) instead of a hand-written table, and the paths
  and verbs come from the mounted router. The wire format is unchanged. The document changed in these ways only:
  the success response of each operation says `OK` rather than a type name; `GET .../subject-access` now lists the
  `external_links` it always returned; `GET /api/nurture/advance` documents its `held` count; `/api/events/recent`
  documents the four members both transports return; and the segment filter, break-glass grants and masked amounts
  are documented as nullable where the service returns `null`.
- The campaign, ticket, article and partnership-stage status bodies are one shared type (`StatusPayload`).

### Added — tests for the middlewares and the request paths (CRM-T82)

- The six middlewares moved out of `app.rs` into `src/middleware/`, with unit tests for the rate limit, `no-store` and
  idempotency answers, and request tests for contacts, accounts, activities, events (both transports) and the rate limit.

### Changed — GDS API standards (CRM-T49)

- **Unknown request members are refused.** Every JSON request body (the `*Payload` structs) now answers `422` for a
  member it does not know, so a misspelt field is no longer silently dropped. Under `api-lifecycle.md` this narrows what
  an endpoint accepts: a client that sends extra members will now fail. The webhook body and query parameters are not
  strict. The `422` names the problem (`unknown field `x`, expected one of …`): request bodies now go through
  `src/json.rs`, which keeps the parser's message that Loco's own `Json` extractor discards.
- **`Cache-Control: no-store` on every `/api/*` response**, errors included (the bodies hold personal data).
- **`RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset` headers** on responses when the request limit is on
  (always in production), so a client sees its budget before it gets a `429`; the `429` carries them too.

### Changed — insights are computed in the database (CRM-T80)

- The cadence, engagement, members, stakeholders, pipeline-hygiene, DPO and consent-by-account views no longer load
  every activity or consent event into memory; the database returns one row per subject or group. The answers are
  the same. Pipeline hygiene was quadratic in the number of deals and now is not, and the sales dashboard no longer
  makes a query per open deal.
- `GET /api/contacts/{pid}` returns at most 200 activities, deals and tickets each, newest first, and says which were
  cut in a new `truncated` member. Deals and tickets were unbounded; activities were cut at 200 without saying so.

### Added — validation errors name their member (CRM-T79)

- A `422` validation error now has an `errors` list, `[{ "pointer": "/display_name", "detail": "…" }]`, beside the
  unchanged `error`, `description` and `detail`. `pointer` is a JSON Pointer to the request member and is left out
  when the problem is about the request as a whole. For a body the parser refused, it follows the parser's path
  (`/stages/2/probability_percent`). The OpenAPI `Error` schema documents it.
- Break-glass refusals, lifecycle transitions and partnership transitions are typed errors internally; the messages
  clients read are the same.

### Changed — settings in one place (CRM-T78)

- The `CRM_*` settings that were read lazily from the environment (authentication switches, key and policy sources,
  the event transport, the webhook and Mailgun secrets, the mail batch, the retention horizon, the Redis URL and
  others) are parsed once into `settings::Settings`. Values and defaults are unchanged. The webhook secret, Mailgun
  key, retention days and mail batch were read on every call and are now read once per process.

### Removed — unused JWT configuration (CRM-T85)

- The `auth.jwt` blocks in `config/*.yaml` are gone. Nothing read them (loco's JWT support is not compiled in), and the
  production one made the service refuse to start without `JWT_SECRET`. Once this is released, `JWT_SECRET` is no
  longer needed; releases up to 0.1.5 still need it. The development and test files no longer hold literal secrets.

### Changed — the display-name cache is bounded (CRM-T81)

- Looked-up display names are kept for 10 minutes and at most 10,000 at a time (the oldest goes first). They were
  kept, unbounded, for the life of the process. A rename or erasure upstream now shows within 10 minutes.

### Changed — dependencies (CRM-T84)

- **reqwest 0.13** (was 0.12) for the email-provider and upstream clients. Certificates are now checked against the
  operating system's trust store instead of bundled webpki roots, the same as the token verifier's key fetch already did.
- **Matcher 0.1.2** (was 0.1.1): the `const fn` change only, no behaviour change.
- **Removed** `tracing-subscriber` (unused), and the SQLite driver from the binary (tests still use it).

## [0.1.5] — 2026-10-21

No service code changes. The service image is now built for amd64 and arm64 (each on a native runner)
and published as one multi-arch image (CRM-T74); proven by the pre-release `v0.1.5-rc.1`.

## [0.1.4] — 2026-10-21

### Security — `rustls-webpki` 0.103.16 (RUSTSEC-2026-0382)

A certificate-verification advisory published on 2026-10-20; `Cargo.lock` now pins the fixed version.

### Fixed — a `--worker` container sent campaigns with the simulated sender (2026-10-20)

`mail::init`, the unsubscribe links and the field-encryption check ran only in `after_routes`,
which a process started with `--worker` never reaches. A worker-only container therefore used the
simulated sender: it marked every recipient sent and the campaign delivered, and no mail left.
Found by the new image smoke test against the released v0.1.3 image (24 of 27 checks passed; the
three failures were this). `init_sending` now runs from both `after_routes` and `connect_workers`.
Affects v0.1.3 and earlier when `CRM_MAIL_MODE=smtp` runs the sending in a separate `--worker`
container (the arrangement the runbook describes). With `--server-and-worker` in one process, or the
simulated default, nothing changes. A DB-backed test (`a_worker_only_process_sets_up_the_real_sender`) now
calls the worker hook after resetting what `after_routes` set, and fails without the fix.

### Added — `examples/mint_token.rs`

Mints a PASETO `v4.public` token and the matching key set from a fixed, public seed, for trying the
service with authentication on. Used by `scripts/image-smoke.sh`. Never trust that key anywhere real.

## [0.1.3] — 2026-10-20

### Added — links to records in other systems (CRM-R27)

A contact, account, deal or lead can carry up to 50 links to the same record in HubSpot,
Salesforce, Attio, Pipedrive, Dynamics or elsewhere: `GET` and `POST
/api/{contacts,accounts,deals,leads}/{pid}/external-links`, `DELETE
/api/external-links/{pid}`. Only a link: nothing is fetched or synchronised. Addresses must
be `https`, without credentials, and on the named system's own domains. Contact links move
with a merge, come back with an unmerge, appear in the subject-access export and are
deleted by erasure. Migration `m20261020_000019_external_links`.

### Added — `POST /api/contacts/{pid}/unmerge` (CRM-R26)

Also `GET /api/contacts/{pid}/merges`, the contacts merged into this one that can still be undone.
Undoes a merge from its ledger: rows still on the survivor move back (anything since
moved on is left behind and counted), the dropped nurture enrolments and group seats are
restored, filled fields are cleared where the survivor still holds the value it was given,
and the contact is live again. Consent never widens (the returning contact gets the
stricter of its own state and the survivor's). `dry_run` previews. Destructive-classified
(`/unmerge`), audited, emitted. Migration `m20261020_000018_contact_unmerge`: the ledger
gains `before`, `unmerged_at`, `unmerged_actor`, and the unique index on `merged_pid` is
now partial so a contact can be merged again. Merges made before it cannot be undone.

### Fixed — erasing a survivor now scrubs the contacts merged into it

`/erase` left a merged-away duplicate's display name and job title in its soft-deleted
row until the retention sweep, and the merge ledger kept its `person_ref`. Both are
scrubbed with the survivor.

### Changed — `controllers/insights.rs` split into `controllers/insights/` (2026-10-20)

The 1,221-line file is now `mod.rs` (routes, the `live!` macro and the shared query helpers),
`sales.rs`, `support.rs`, `compliance.rs`, `outreach.rs` and `relationships.rs`. No behaviour
change; `controllers::insights::routes` is unchanged.

### Changed — `controllers/marketing.rs` split into `controllers/marketing/` (2026-10-20)

The 1,659-line file is now `mod.rs` (routes and the shared `{pid}` response), `consent.rs`,
`segment.rs`, `campaign.rs`, `nurture.rs` and `public.rs` (unsubscribe and the email
webhooks). No behaviour change; paths such as `controllers::marketing::routes` and
`CampaignSendWorker` are unchanged.

### Changed — `src/auth.rs` split into `src/auth/` (2026-10-20)

The 1,955-line file is now `mod.rs` (the guide and re-exports), `verifier.rs`,
`policy.rs`, `records.rs`, `extract.rs` and `tests.rs`. No behaviour change and every
`crate::auth::...` path is unchanged. Builds and lints clean with and without the
`paseto` and `keycloak` features.

## [0.1.2] — 2026-10-20

Since 0.1.0 (crates.io) and the v0.1.1 images: `GET /api/contacts/{pid}/duplicates`
(below), a dependency on the published `contact-relationship-management-matcher`
0.1.1 (the 0.99 match threshold), the Redis tests now run in CI, the images carry OCI labels and the release
workflow refuses a tag that disagrees with this version, and the UI image builds
on Node 26 (v0.1.0's UI build failed on `corepack`). The release workflow now uses
cosign-installer 4 (cosign 3) and attest-build-provenance 4: `v0.1.2-rc.1` ran it and both images
verify with cosign 3 (cosign 2 clients cannot verify the new signatures).

### Fixed — a write no longer waits on a dead Redis (CRM-R25)

With Redis stopped or frozen, a successful write waited for the Redis pool's own
timeout (over 20 seconds) before answering, because the insight cache's
generation bump after every write had no limit. Every insight-cache call is now
bounded to 750 ms and its failure only skips the cache. Found by the new
`scripts/chaos.sh`.

### Added — `POST /api/contacts/{pid}/merge` (CRM-R24)

Merges the contact in the path into `into_pid`, which survives. One transaction;
`dry_run` does the work and rolls it back, so its counts are exact. Activities,
leads, deals (as primary contact), tickets, email deliveries, consent events,
nurture enrolments and working-group seats move to the survivor (an enrolment or
seat the survivor already has is dropped). The survivor's empty fields are filled
from the merged contact; nothing it holds is overwritten. Consent is the stricter
of the two (`withdrawn`, then `never`, then `granted`) and a consent event
records any change. The merged contact is soft-deleted with `merged_into_pid`; a
new `contact_merges` ledger holds the ids of every row that moved. Refused: the
same contact (422), an erased contact (422), an already-merged one (409), a
missing one (404), a caller who may not do destructive actions (403). Audited
(counts, never values) and emitted as `contacts_merged`. New migration
`m20261019_000017_contact_merges`.

### Added — `GET /api/contacts/{pid}/duplicates` (2026-10-19)

Other contacts that are probably the same person, best first, with the
probability and a per-field breakdown (CRM-R23, CRM-T61). It uses the new
`contact-relationship-management-matcher` crate, published as 0.1.0 for this.
First pass on name, job title, account name and `person_ref`; the best 20 then
have their email addresses fetched from the person service and are rescored.
Record-level access applies and the read is audited as a count. At most 5,000
contacts are compared. It is advice: nothing is merged or deleted.

### Changed — `authentication-verifier` from crates.io (2026-10-18)

The dependency (and its dev-dependency with `test-idp`) is now the published
0.9.1 instead of a pinned git commit, which crates.io would reject. Tests,
clippy (with `--features keycloak`) and `cargo deny` pass. The migrations crate
is now `contact-relationship-management-migration` (still imported as
`migration`, via `[lib] name` and a renamed dependency) and is publishable; the
service refers to it by version `0.1.0` plus its path. `cargo publish
--workspace --dry-run` passes for both crates. Nothing has been published.
The package ships only `src/` and the README (`include` in `Cargo.toml`), so it
carries no `config/*.yaml`, tests or Containerfile.

### Changed — crate renamed (2026-10-18)

The package is now `contact-relationship-management` (was
`contact-relationship-management-service`), so the library is
`contact_relationship_management`. The binary (`[[bin]]`, `default-run`, the
Containerfile) is now `contact-relationship-management` too, and the default
databases are `contact_relationship_management_development` and
`..._test` (were `contact_relationship_management_service_…`); CI uses
`ci_contact_relationship_management`. **Existing local databases must be
renamed** (`ALTER DATABASE contact_relationship_management_service_development
RENAME TO contact_relationship_management_development;`, same for `_test`), or
point `DATABASE_URL` at the old name. The package description was also reworded.

### Fixed — `updated_at` now moves on every edit (CRM-T57, 2026-10-18)

SeaORM updates write only the columns given, so `updated_at` kept its insert
value: an edited deal said it was last changed when it was created. Migration
`m20261018_000016` adds a `BEFORE UPDATE` trigger on the 18 tables that are
edited in place; it sets `now()` unless the statement sets `updated_at` itself.
Append-only tables (audit log, consent events, forecast snapshots, outbox, queue)
are left alone. A lead rename in the database tests checks it.

### Added — board edits and follow-up reschedule (CRM-T55, 2026-10-17)

`PUT /api/deals/{pid}`, `/leads/{pid}`, `/tickets/{pid}` edit a card in place;
`PUT /api/activities/{pid}` changes a follow-up's due day (audited as
`rescheduled`) or summary; `PUT /api/{deals,leads,tickets}/{pid}/position` stores
the card's place in its column (migration `m20261017_000015` adds
`board_position` to leads and tickets; deals already had `kanban_position`).
Last write wins; there is no version check.

### Added — real email send path (CRM-T39, 2026-10-11)

`mail::Sender` (simulated by default, SMTP with `CRM_MAIL_MODE=smtp`), an
`email_deliveries` table with one row per campaign and contact (atomic claim,
retry, consent re-read before each send), campaign `subject` and `body`, and
`GET /api/campaigns/{pid}/deliveries`. Production refuses `smtp` mode without
`CRM_MAIL_ALLOW_PRODUCTION=1`. Not built: address-level suppression list,
bounce and complaint webhooks, nurture sends.

### Added — SES, Azure, Mailgun and SendGrid adapters (CRM-T39, 2026-10-13)

`CRM_MAIL_MODE=ses|azure|mailgun|sendgrid` (`src/provider_cloud.rs`,
`src/provider_api.rs`) and `POST /webhooks/email/{ses,azure,mailgun,sendgrid}`.
Migration `m20261013_000014` adds `email_deliveries.provider_id` for Azure,
whose events name the message by the id returned at send. Mailgun callbacks are
verified with `CRM_MAILGUN_SIGNING_KEY`. SNS and SendGrid signatures are not
verified. Not run against live accounts.

### Added — Brevo and Mailjet adapters (CRM-T39, 2026-10-12)

`CRM_MAIL_MODE=brevo` or `mailjet` sends through the vendor's HTTP API
(`src/provider.rs`), tagging each message with our id (Brevo `tags`, Mailjet
`CustomID`) so the vendor's events can be matched. `POST
/webhooks/email/{brevo,mailjet}` reads their callbacks (delivered, bounces,
spam) into the generic outcomes, authenticated by `CRM_WEBHOOK_SECRET` as a
bearer token, basic-auth password or `X-CRM-Webhook-Token`. Checked against the
vendors' documentation only; not run against a live account.

### Added — real nurture sends (CRM-T39, 2026-10-12)

Nurture steps carry an optional `subject` and a plain-text `body`
(migration `m20261012_000013_nurture_step_content`). With a real sender,
`POST /api/nurture/advance` mails each due step after the consent and
suppression checks, once per (step, contact), with the unsubscribe link, and
reports `held` for enrolments it will retry. The person record's mailbox is read
from `email` or `emails` (`clients::extract_email`).

### Changed — campaign sends run as a background job (CRM-T39, 2026-10-12)

With a real sender, `run` validates the campaign and enqueues
`CampaignSendWorker`, which sends batch after batch until nothing more can be
sent. It answers `202` until the campaign completes. A pass that sends nothing
stops the job; a later `run` resumes. Simulated mode is unchanged. The worker
is registered in `connect_workers`.

### Added — lifting a suppression (CRM-T39, 2026-10-12)

`GET` and `DELETE /api/contacts/{pid}/email-suppression` read and lift a
mailbox suppression. Both are audited; lifting does not restore consent.

### Added — outcome webhook and suppression list (CRM-T39, 2026-10-11)

`POST /webhooks/email` (HMAC-signed, `CRM_WEBHOOK_SECRET`) records provider
outcomes once per message and event. `delivered` counts on the campaign; a hard
bounce or complaint suppresses the contact (`email_suppressions`, skipped by
the send path), and a complaint withdraws consent. Migration
`m20261011_000012_email_events`.

### Added — unsubscribe link and headers (CRM-T39, 2026-10-11)

Every real message ends with a signed unsubscribe link and carries
`List-Unsubscribe` and `List-Unsubscribe-Post` (RFC 8058). `GET
/unsubscribe/{token}` changes nothing; `POST` withdraws marketing consent
through the ordinary audited consent change and is idempotent. Public (no
bearer token), HMAC-SHA256 signed, no expiry. `smtp` mode needs
`CRM_UNSUBSCRIBE_SECRET` and `CRM_PUBLIC_URL`. A local Stalwart server for
development is in `compose.email.yaml` and `scripts/email-dev.sh`.

### Changed — RFC 9457 error bodies (CRM-T54, 2026-10-11)

Every `4xx`/`5xx` is now `application/problem+json` with `type`, `title`,
`status`, `detail` and `instance`. The earlier `error` and `description`
members are kept, so only the media type changes. A `401` carries
`WWW-Authenticate: Bearer`. The API versioning and deprecation policy is in
`spec/compliance/api-lifecycle.md`.

### Added — response schemas (CRM-T53, 2026-10-11)

The OpenAPI document types the detail reads, forecast, dashboards, insight
views, audit, retention, privacy and break-glass responses; tests validate
real (including masked) responses against them.

### Added — field-level encryption (CRM-T41, 2026-10-10)

AES-256-GCM on `contacts.preferred_channel` and `activities.summary` through
an `EncryptedText` column type, keyring with rotation (`CRM_FIELD_KEYS`),
`rotate_field_keys` task, and `CRM_REQUIRE_FIELD_ENCRYPTION` start-up check.

### Added — break-glass access (CRM-T44, 2026-10-10)

Reasoned, time-limited, audited read elevation (`/api/break-glass`) with a
review queue; reference policy lifts the mask for reads while a grant is live.

### Added — abuse resistance (CRM-T43, 2026-10-10)

Per-client request limit and failed-authentication lockout (`429` with
`Retry-After`), on by default in production; probes exempt. The UI signs out
an idle session after 15 minutes.

### Added — access-model completion (CRM-T32, T38, T40, 2026-10-10)

`mask` now covers the forecast, sales dashboard, campaign cost/funnel and the
rows bundled into a contact read. Audit, event and retention reads are
admin/service tier; `CRM_PROTECT_OPERATIONAL=1` also protects metrics and
OpenAPI. Personal-data reads (contacts, leads, tickets, activities) are
audited into the hash chain.

### Added — production-readiness hardening (CRM-T30, T31, T33, T34, T35, 2026-10-10)

Production boot refuses to start with `CRM_REQUIRE_AUTH` off
(`CRM_ALLOW_INSECURE=1` overrides). `audit_logs` is now a tamper-evident hash
chain with an append-only trigger and `GET /api/audits/verify`
(migration `m20261010_000008_audit_chain`). `config/production.yaml` adds
security headers, a closed CORS list and no `x-powered-by`, and its SMTP
credentials no longer break parsing when empty. `deny.toml`, a hardened
`Containerfile`, and a CI `supply-chain` stage (audit, deny, SBOM).

### Added — optional Keycloak bearer acceptance; PASETO is now a feature (KC-1, 2026-09-29)

`keycloak` cargo feature (off by default): the blanket guard and the
`AuthUser` / `MaybeAuthUser` extractors also accept an
`Authorization: Bearer <Keycloak access token>`, verified by
`authentication-verifier`'s `keycloak` module and mapped onto the same
`Claims`, so ABAC, masking and audit are unchanged. Configured by
`CRM_KEYCLOAK_URL` / `_REALM` / `_AUDIENCES` / `_ROLE_MAP[_FILE]` /
`_REQUIRE_VERIFIED_EMAIL`; unset URL => off; a malformed configuration logs
an error and leaves Keycloak off (fail-closed, service still boots). A token
beginning `v4.` is a PASETO; anything else is offered to Keycloak. An
unmapped Keycloak role grants nothing. New `auth::request_claims` and
`auth::enforce_request` (async) are what the middleware and extractors use;
the sync `enforce` / `bearer_claims` remain for the PASETO tests.

PASETO verification (`Verifier`, the boot key fetch, the rotation refresh)
moved behind the `paseto` feature, **on by default**, so a default build is
unchanged. `--no-default-features --features keycloak` builds a service with
no PASETO code. New `tests/keycloak_guard.rs` (DB-free) pushes real RS256
Keycloak-shaped tokens, served by a local OIDC provider
(`authentication_verifier::test_idp`), through the real guard: editor may
read/write but not delete/merge, admin may, unmapped roles are read-only,
expired / wrong-audience / garbage / PASETO-shaped-junk are 401. See
`agents/share/runbooks/keycloak-sso.md`.

### Fixed — record-level ABAC wired into the contact/deal handlers (CRM-T24)

`auth::deal_resource_attrs`/`auth::contact_resource_attrs` were defined
and unit-tested but `authorize_record` was called only from
`controllers/privacy.rs` (subject-access/erase) — the deal and contact
read/write handlers ran no record-level check at all. `get_contact` and
`list_contacts` (via a new `readable_contacts` helper, omitting a
denied row per invariant 5) now call `authorize_record` and, on a
`mask` obligation, redact `preferred_channel` through a new
`auth::mask_text_json` (a text-placeholder sibling of the existing
amount-nulling `auth::mask_json`); `repoint_contact` gates its write
the same way. `list_deals` (via a new `readable_deals` helper) and
`deal_stage`/`reopen_deal` do the equivalent for deals, redacting
`amount_minor`. New unit test `mask_text_json_redacts_text_keeps_structure`;
`tests/enforcement.rs` gained an owner-vs-non-owner,
masked-vs-unmasked scenario for one deal and one contact endpoint.
`get_contact`'s bundled activities/deals/tickets rows, and the
forecast/ROI fields CRM-T22 also named, are explicitly left out of
this change (documented, not silent). See spec/tasks.md CRM-T24.

### Added — `require_ref` test coverage for Worker and Organization (CRM-T25)

`src/validation.rs`'s `ref_rules` unit test only ever exercised
`EntityType::Person`, even though `controllers/sales.rs`,
`support.rs`, and `relationships.rs` all call the shared `require_ref`
helper with `EntityType::Worker`/`::Organization` too. New
`ref_rules_wrong_type_worker_and_organization` test pins both the
wrong-type rejection and the matching-type acceptance for each. See
`../spec/tasks.md` CRM-T25.

### Added — declared MSRV (Rust 1.95)

- `Cargo.toml` now declares `rust-version = "1.95"`, the repository's
  **current stable minus three** floor
  (`spec/rust-msrv-n-minus-3/index.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.95 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption.
  *(Corrected 2026-09-06: `Cargo.toml` now declares `rust-version =
  "1.96"`, matching `ci/msrv.txt` and the repository's current **N-2**
  policy (`spec/rust-msrv-n-minus-2/index.md`) — the policy tightened
  from N-3 to N-2 after this entry was written. No behaviour change;
  `Cargo.toml`, `ci/msrv.txt`, and `scripts/ci-check.sh msrv` already
  agreed on 1.96 before this correction.)*

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration →
  2.0, sea-query → 1.0. No feature-list changes (default feature set).
- **`ColType::PkAuto` now generates a 64-bit primary key.** Of this
  crate's ~23 tables, exactly one (`audit_logs`) goes through loco's
  schema DSL and moves from `i32` to `i64`; the relationships, sales,
  marketing, support, engagement, and `event_outbox` tables are all
  created with raw SQL and stay `i32` — same split as the sibling
  consumer apps.
- A `useless_conversion` in `src/models/event_outbox.rs` and a
  pre-existing `needless_borrows_for_generic_args` in
  `src/controllers/mod.rs`, both surfaced by the same clippy run.
- No behavioural change; verified with the full DB-gated suite (8
  tests, unchanged count) against a freshly migrated Postgres 18.

### Added — engagement / partnerships / confederation (CRM-T20, 2026-07-20)

- Declared stakeholder typing (role + power–interest 1–5 on contacts;
  role on accounts), recorded activity sentiment, the forward-only
  partnership lifecycle, per-account memberships, and working groups
  with derived activity feeds (`controllers/engagement.rs`).
- Nine new derived views (`controllers/insights.rs`): cadence,
  engagement workload, the audit-derived pipeline funnel, member
  health, consent-by-account, the stakeholder register + grid, the
  partnership register, membership renewals; follow-ups gains a
  `kind` filter (renewals convention).

### Added — insight views (CRM-T19, 2026-07-20)

- `controllers/insights.rs`: stale-deal aging (from stage-change
  audits), follow-ups (overdue + 30-day horizon), pipeline-hygiene
  findings, the period executive pack (per-currency won value never
  merged), the stored forecast-trend series (no interpolation), the
  SLA breach register + per-assignee workload, and the DPO view
  (consent coverage + withdrawals + per-source counts +
  duplicate-contact hygiene). All ETag-conditional with `as_of`.

### Added

- 2026-07-18 — CRM-T1–T16 implementation round: full Loco service
  (copy-adapted from the WPM service). 6 migrations / 19 tables,
  pure `rules/` core (lifecycle machines, deterministic lead scoring
  with breakdown, forecast/ROI/CLV/win-rate with per-currency
  honesty, SLA derivation, consent-gated segment evaluation), five
  module controllers (relationships / sales / marketing / support /
  dashboards) + audits/docs/metrics, offline PASETO + ABAC with
  `resource.owner` `$sub` ownership + amount masking,
  ETag-conditional dashboards, idempotent nurture advance + SLA
  sweep, simulated campaign send (consent re-checked at send time),
  seed task. 62 unit + 5 request + 1 enforcement tests green against
  Postgres 18; clippy-pedantic clean; live smoke verified.

- 2026-07-18 — CRM-T0 specification round: cross-cutting spec
  (`../spec/`) with the four-module domain, SDD trio
  (requirements CRM-R1–R17, design CRM-D1–D12, tasks CRM-T*), and
  this edition's doc scaffold. No code yet.
