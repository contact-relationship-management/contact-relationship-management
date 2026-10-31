# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — the consent-profile tables and columns (CRM-T166 step c, CRM-D21)

Migration `000059`: `consent_jurisdictions` (code, layer, parent, name); `consent_profile_rows` (code, key, JSON value, `effective_from`/`effective_to`, source, `checked_on`, status `read`/`unverified`/`example`, who recorded it; append-only, a trigger refuses an update or a delete; `id` is the recording order); `consent_profile_acknowledgements` (code, `through_id`, who and when; append-only); `consent_purposes` (key, position, names and descriptions per locale, `active`, `soft_opt_in_eligible`), seeded with today's four purposes at the positions signed links already name them by; `channel` on `contact_consents` (now keyed by contact, purpose and channel) and on `consent_events`, every existing row `email`; `consent_jurisdiction` on contacts, campaigns and nurture sequences; `sender_name`, `postal_address` and `contact_link` on `organization_settings`; `basis` on `email_deliveries`. The entities of contacts, campaigns, nurture sequences and consent events carry the new columns (so the OpenAPI document and the UI's `schema.d.ts` do); the consent reads and writes name the `email` channel. The personal-data inventory lists the four new tables. **Not done or verified:** nothing reads the new tables or columns yet: every send still checks only `granted`; the migration's `down` was not run; no request test of the new columns beyond the whole suite passing (223 request tests against a throwaway PostgreSQL database).

### Added — the pure consent-regime core (CRM-T166 step b, CRM-D21)

`rules::consent_regime` (DB-free, clock-free), a copy of the workforce planning service's `rules::jurisdiction` (WPM-D144) adapted to the CRM's keys: jurisdiction codes, layers and chains; the parameter registry (`consent.regime.<channel>`, `consent.soft_opt_in.window_days.<channel>`, `consent.implied.expiry_days.<inquiry|won_deal>`, `message.sender_identification`, `message.postal_address`, `message.advertising_label`) with a combination rule per key (`strictest` for regimes and requirements, `strictest_max` for windows and expiries, `most_specific` for the label); dated, append-only rows with a status; `resolve`; `rules_for` (a channel's regime, window and expiries, where an `example` or `unverified` row loosens only when an acknowledgement covers it, and an unacknowledged row can only tighten); `message_rules`; `basis` (granted, soft opt-in, implied with the evidence and its expiry, opt-out, or the reason it is blocked; `pending` treated as `never` under soft opt-in and implied); the `XA`, `XB` and `XB-01` example profiles. 14 unit tests, including the invariant that with no profile every state's answer equals today's. **Not done:** nothing calls it yet (no table, send path, API or UI); the registry is not checked against the spec's table by a test; no law was read.

### Added — whose each retention default is; the access export names whose right it serves (CRM-T165, CRM-D19)

`rules::privacy::RETENTION_DEFAULTS` (with `DefaultFrom`, the shape of the workforce planning service's `default_from`, WPM-T384): `CRM_RETENTION_DAYS`, `CRM_EVENT_RETENTION_DAYS` and `CRM_MAIL_RETENTION_DAYS` with their defaults (365, 30, 730), floors (30, 7, 400) and `default_from` (`regime: null`, a `reference`, `status: starting_point`). `GET /api/retention` answers `horizon_basis` (setting, days in force, `set`, default, floor, `default_from`); its `derivation` says no default is any jurisdiction's. The OpenAPI summary of `GET /api/contacts/{pid}/subject-access` names the contact's right of access under the data-protection law that applies to the deployer (GDPR Art. 15 as an example), not "GDPR subject-access export". Tests: two unit tests (the table equals the horizon constants; the served summary, and no summary starting with one law's name) and the privacy request test reading `horizon_basis`. **Not done:** the sweep's answer has no `horizon_basis`; the horizons are still one value per deployment, not per jurisdiction.

### Added — the stalled-deal flag (CRM-T183, CRM-R4, CRM-D20)

The pure `rules::stalled` (`days`, `last_touch`, `is_stalled`). Migration `000058`: `deals.stage_moved_at` (filled from the newest `deal_stage_changed`, `deal_won`, `deal_lost` or `deal_reopened` audit row, else `created_at`, without moving `updated_at`) and `organization_settings.stalled_after_days` (1 to 365, default 14). `POST /api/deals/{pid}/stage` and `/reopen` set `stage_moved_at`. `GET /api/deals` answers `ListedDeal` (the deal plus `stalled` and `last_touched_at`; the last live activity on each deal that has happened, in one grouped query). `GET`/`PUT /api/settings/organization` carry `stalled_after_days` (a `PUT` without it keeps it; audited). Tests: 3 unit, 1 migration unit, request test `a_deal_with_no_move_or_activity_for_n_days_is_stalled`. **Not done or verified:** activities on the deal's contact or account do not count; the contact bundle and the stage-move answer carry no flag; the stale-deals insight still reads the audit log; the seed has no stalled deal; migration `000058` not timed on a large table; not run with the UI against a live service.

### Changed — matcher 0.2.0 (CRM-T87, CRM-T169, CRM-T170)

`contact-relationship-management-matcher` `0.1.2` → `0.2.0` from crates.io; `Cargo.lock` updated (cargo also moved eight Windows-only `windows-sys` dependency lines from 0.59.0 to 0.61.2, both already in the lock). The breaking changes (`#[non_exhaustive]` on `Field`, `Outcome`, `Verdict`, `MatchResult` and `FieldScore`; `ConfigError` an enum) needed no code change: the service compares a `Verdict` with `==` and never matches any of them exhaustively, and never names `ConfigError`. The duplicate check, the import duplicate hints and relationships now use the matcher's per-language words and its phone rules. **Not done or verified:** no release; not run against a live service with the UI.

### Changed — the web form's `422`, `403`, `429` and `503` in the request's language (CRM-T182, CRM-R31)

`rules::public_text::Text` gains `form_invalid_title`, `form_name_invalid`, `form_email_invalid`, `form_refused_title`, `form_refused`, `form_busy_title`, `form_busy`, `form_unavailable_title` and `form_unavailable` (eight languages, machine drafts but English). `controllers::forms` chooses the language before the origin check (`language_of`) and answers every refusal in it through `in_language` and `refusal`, with `Content-Language` and `Vary: Accept-Language`; the `error` codes stay `validation`, `forbidden`, `too_many_requests` and `unavailable`. `problem::Title` lets a response set the problem document's `title`. The preflight's `403` follows `Accept-Language`. OpenAPI's `/forms/{slug}` description updated. Request test `the_form_s_refusals_are_in_the_language_of_the_request`. **Not done or verified:** the translations are unreviewed; a body the JSON reader refuses is answered in English before the form sees it; the `404` of an unknown form has no text; no browser check.

### Fixed — the days deals close on are the organization's (CRM-T181, CRM-R48, CRM-R70, CRM-R83, CRM-R85, CRM-R86)

`rules::periods` gains `day_of` and `local_day`. `deals.closed_at` (a `TIMESTAMPTZ`) was taken as a day by its UTC date (`date_naive()`, `closed_at::date`); its day in the organization's zone now decides the close-day rate in `GET /api/accounts/{pid}/clv`, `GET /api/campaigns/{pid}/funnel`, `GET /api/attribution` and `POST /api/reports/run` (`rate_basis=close_day`), attribution's `from`/`to`, `quotas::credited_deals` (quota and team attainment) and territory attainment's `from`/`to`. Unit test `an_event_s_day_is_the_organization_s`; request test `a_deal_closes_on_the_organization_s_day` (at `Pacific/Kiritimati`). **Not done or verified:** no request test of quota or team attainment in a zone; the win-loss insight's window and the CLV look-ahead's history years are elapsed-time windows and were not changed; `expected_close` is a stored date and is unchanged; not run with the UI against a live service.

### Changed — the web form's confirmation mail and answer in the request's language (CRM-T180, CRM-R29, CRM-R31)

`rules::public_text::Text` gains `mail_subject`, `mail_body` and `form_received` (eight languages, machine drafts but English) and `confirmation_mail`; `controllers::forms` takes an optional `language`, negotiates it with `Accept-Language` (`language_of`), sends the confirmation in that language with `?lang=` on its link, and answers `202` in it with `Content-Language` and `Vary: Accept-Language`. `controllers::marketing::purpose_label` is removed. OpenAPI: `/forms/{slug}` documents `language`. Request test `the_confirmation_mail_is_in_the_language_of_the_request_that_asked_for_it`. **Not done or verified:** the translations are unreviewed; the form's error answers stay English; no preference mail is sent and the unsubscribe links of campaign and nurture mail do not name the template's language; not sent through a real provider.

### Fixed — the remaining "today"s in the organization's or the caller's zone (CRM-T179, CRM-R48, CRM-R94)

`controllers::organization::caller_today` now decides `GET /api/insights/followups`, `pipeline-hygiene` and `memberships`; `middleware::insight_cache::insight_key` takes the `CRM-Time-Zone` header. `organization_today` decides `kb_daily.day` (was `CURRENT_DATE`), the `kb-deflection` window (its ticket count by `opened_at` in the organization's zone, was `opened_at::date`), a campaign funnel's `rate_date` when nothing is converted, a workflow `create_task`'s `due_on`, the employment check's `since` (CRM's own record day; WPM-R208's answer has no date), attribution's unshown placeholder day and the seed's rate day. Request test `the_other_todays_follow_the_organization_or_the_caller`. **Not done or verified:** stored `kb_daily` days are not moved; deal close days (close-day rates, attribution's `from`/`to`) stay UTC dates; no request test of the memberships window or the employment check in a zone; not run with the UI against a live service.

### Added — the organization's calendar: time zone, fiscal year, "today" and report periods (CRM-T171, CRM-R5, CRM-R48)

`rules::periods` (pure); migration `000057_organization_settings`; `controllers::organization` with `GET`/`PUT /api/settings/organization` (the `PUT` admin-only in `auth::policy`, audited), `periods`, `caller_today` (the `CRM-Time-Zone` header, else the organization's zone) and `organization_today`. Attainment (quota, team, territory) and import date checks use the caller's today; the forecast snapshot and `fx_rates::convert_sums` the organization's. `rules::reports::plan_in` resolves `@month`, `@fiscal_quarter` and `@fiscal_year` fields in the organization's zone and fiscal year, adds `*_fiscal_quarter` and `*_fiscal_year` to every kind, and reads `from`/`to` as the zone's days; `ReportResult` carries `time_zone` and `fiscal_year_start_month`. Tests: 7 unit, 1 policy, 1 request test. **Not done:** the KB, attribution, WPM bridge, campaign and workflow "today"s still read UTC; no week groups.

### Changed — the public consent pages in the reader's language; their error pages stay HTML (CRM-T175, CRM-R29)

`rules::public_text` (pure) holds the confirmation, preference and unsubscribe pages' words in the UI's eight languages (machine drafts but English) and `negotiate`, which picks the address's `?lang=`, else the best `Accept-Language` range by `q` weight, else English. Every public page carries `<html lang dir>` (Arabic `rtl`), `Content-Language`, `Vary: Accept-Language`, links to itself in the other seven languages (each named in its own), and forms that keep `?lang=`. `purpose_label` (the web form's answer) is the English words from the same table. **Fixed on the way:** the problem-document middleware turned the public pages' `404`, `422` and `500` HTML into JSON, so a person with an expired link read a problem document; a response marked `problem::PublicPage` (set by `public_page`) now passes through, while every other error, the router's own HTML 404 among them, is still a problem document. Tests: 4 unit tests of `public_text`, and the request test `the_public_pages_are_in_the_reader_s_language` (German by weight, Arabic right to left by `?lang=`, a Chinese answer to the button, English for Korean, Welsh on the unsubscribe page, French preferences keeping `?lang=`). **Not done:** the contact's language is not stored (on purpose; the reasons are in CRM-R29); the confirmation and preference mails and the web form's own answer are still English; languages beyond the eight fall back to English; the drafts are unreviewed.

### Changed — template fallbacks in the template's language; `given_name` never guessed; import columns in eight languages (CRM-T169, CRM-R36, CRM-R38, CRM-R34, CRM-R43)

`rules::templates`: `first_word` is gone and `given_name` fills from the stored given name only; `Token` loses its English `fallback` for `FALLBACKS` (the UI's eight languages), `fallbacks_in`, `built_in_fallback`, `language` (the BCP 47 shape) and `UNNAMED_LANGUAGE` (`en`); `check` takes the template's language and asks for the template's own fallback for each token used when the language has none built in; `Fill` carries the language. Templates take and return `language` (migration `000056_template_language`, `VARCHAR(35) NULL`); `GET /api/email-templates/tokens?language=`. `rules::column_names` holds the import header names per field in eight languages; `rules::imports::suggest` and `rules::record_imports::suggest_*` use it. Tests: 7 unit, 1 new request test, 2 request tests changed (a contact without a given name now gets the fallback). OpenAPI updated. **Not done:** no check of the drafts by speakers; a template has one language, not one per recipient; the language is not checked against a registry of real languages, only its shape.

### Fixed — search in every script (CRM-T170, CRM-R33)

`rules::search::words` keeps combining marks; `terms` makes the overlapping character pairs of a word in a script without spaces (`migration::search_docs::UNSPACED`), all required in the `tsquery` (at most 16); `long_enough` takes one ideograph or kana. Migration `000055` creates the immutable `search_ngrams(text)` and rebuilds the seven full-text indexes over it (`search_docs::ngram_index_statements`); the query (`src/search.rs`) reads the same expression; the trigram indexes are unchanged. `unicode-normalization` 0.1.26 is now a direct dependency (it was already in the lockfile). Tests: 3 unit, 1 in the migration crate, and the request test `search_finds_part_of_a_name_in_a_script_without_spaces`; the whole request suite on a fresh PostgreSQL 18 database; migration `000055` up, down and up on a seeded one. **Not done:** no measurement of the index's size or speed; pairs can match across two Thai words; no transliteration.

### Changed — the seed is in more than one currency, rate base and name shape (CRM-T176)

`tasks::seed` takes its deals from `DEAL_AMOUNTS` (GBP, JPY, KWD, EUR, INR, BRL, each in its own minor unit), inserts the six `RATES` for today from five bases (USD, EUR, INR, KWD, JPY; `ON CONFLICT DO NOTHING`, source "synthetic demo rate"), and names the 15 contacts from `CONTACT_NAMES` in Latin, Han, Arabic, Devanagari, Hangul, Cyrillic and Thai, both orders, with `given_name` and `family_name` stored for some and not others. Three unit tests hold the shapes. Run once with `cargo run -- task seed` on a throwaway PostgreSQL 18 database. **Not done:** the accounts, deals and leads keep their "Demo …" names; the request tests do not run the seed; the names were not checked by a speaker of each language.

### Fixed — conversion by each currency's own exponent; currency codes checked (CRM-T167, CRM-T168, CRM-R50)

`rules::currency` (pure): the ISO 4217 minor-unit table, `exponent`, `stored_exponent` and `check`. `rules::fx::convert` multiplies by ten to the target's exponent less the source's in the same division as the rate, so USD 1.00 at 150 is JPY 150 and it still rounds half to even once. `fx::currency` and the new `Problems::currency` check codes against the table; deals, the deal of a lead conversion (`/deal/currency`, before the transaction), campaigns, quotas, rates and `convert_to` answer `422` for an unknown code. `CampaignPayload::currency` is required: the `GBP` default is gone. OpenAPI examples vary (EUR, JPY, INR). Tests: 7 new unit tests (422 in all), `tests/requests/currencies.rs` (3 request tests), and the campaign request bodies that relied on the default now name a currency. **Not done:** no database check on stored codes, and stored codes outside the table count as exponent 2; the seed is still GBP (CRM-T176); not run against the UI or a live deployment.

### Changed — the teams import's lean worker list (CRM-T163, CRM-R73)

`clients::active_workers` sends `fields=pid,department,person_ref` (`WORKER_FIELDS`); the request tests' stand-in WPM projects `fields` and refuses an unknown name with `422`, and a new test shows a WPM without `fields` fails the import with `502` in one call and keeps the existing teams. Not run against a live WPM.

### Changed — dependency update (maintenance, 2026-10-30)

`cargo update`; minimum versions tokio 1.54, uuid 1.29, entity-ref 0.2.1, redis 1.8.0. The axum-test comment said it was the version loco's testing feature uses; it is not (loco uses 17.x, this crate 21.x), and the comment now says so. axum-test 22 is a release candidate and was not taken. Not run: the image build.

### Changed — CRM users are `person:` references; migration 54 (CRM-T162, CRM-D18)

`rules::user_ref` (pure): `of_subject`, `normalise` and friends. `owner_of` and the inbox give `person:<sub>`; `Problems::{require_user, user_opt}` replace the `Worker` checks; every user-naming body and `owner_ref` filter rewrites `worker:<id>` to `person:<id>` first, workflow actions through `Action::normalise_user_refs`. `PUT /api/quotas/teams` takes `person:` members, so an imported team can be saved again, and its roll-up meets its members' quotas. Migration 54 rewrites stored references (23 columns, workflow actions, hand-set teams), skips teams last set by the import and any rewrite that would collide on a unique key, does not move `updated_at` (it switches the touch trigger off for its own updates), and has a `down`. The seed makes `person:` owners; the request tests' helper is `a_user()`. `scripts/migrate-person-refs.sh` checks the migration against populated data. Not run against a live WPM or with real sign-ins; the `worker:` alias has no end date and no `Deprecation` header.

### Fixed — imported team members are `person:` references (CRM-T161, CRM-R73, CRM-D17)

`rules::teams_import::{group, is_person_ref}` (pure); `clients::UpstreamWorker` gains `person_ref`; `POST /api/bridges/wpm/teams-import` stores members by `person_ref`, never `worker:`, and answers `without_person_ref`; the "not a reference" skip for a worker id is gone. No migration: the next import replaces the members of the teams it names. Not run against a live WPM; an imported team's roll-up matches no `worker:`-owned quota.

### Added — WPM employment check (CRM-T160, CRM-R94)

`POST /api/bridges/wpm/employment-check`; `rules::employment_check` (batches, answer, decision); `clients::overlap_check`; `CRM_WPM_ORGANIZATION_REF`; `models::employments::{last_kind, live_person_refs}`. Tested against a stand-in WPM; not run against a live one.

### Fixed — WPM events ordered (CRM-T159, CRM-R93)

`rules::wpm_events::{Stamp, is_stale}`; migration 000053 (`employment_last_events`); `stale` on `POST /api/bridges/wpm/events`; `422` for a hire or leave without `occurred_at`; `employment_last_events` in the subject-access export, deleted on erasure, pruned by the sweep (`aged`). Not run against a live WPM.

### Added — WPM hire and leave events (CRM-T158, CRM-R92)

`POST /api/bridges/wpm/events` (public to the session check, signed with `CRM_WPM_WEBHOOK_SECRET`); `rules::wpm_events` (signature, parse, decide); `models::employments`; migration 000052 (`wpm_events`, `contact_employments`); the segment facts gain `employee`; a campaign skips and a nurture sweep holds an employee; `employments` on the contact detail and the subject-access export; erasure deletes them; `wpm_events` pruned with the event logs. Not run against a live WPM.

### Added — a delay for clicks (CRM-T157, CRM-R91)

`min_click_seconds` on `GET /api/attribution`, validated, echoed and audited.

### Added — position-model weights (CRM-T156, CRM-R90)

`end_percent`; `rules::attribution::credit_tuned`.

### Added — attribution settings (CRM-T155, CRM-R89)

`half_life_days`, `min_open_seconds`; `rules::attribution::credit_with`.

### Added — inbound reply recognition (CRM-T154, CRM-R88)

`POST /webhooks/inbound-email`; `reference` in `GET /api/tickets/{pid}`; `webhook::signed_request`.

### Added — bridge credentials (CRM-T153, CRM-R87)

`ppm_service_token`, `worker_service_token`; `clients::with_credential`.

### Added — close-day campaign ROI (CRM-T152, CRM-R86)

`rate_basis` on the funnel query and answer; the worker import stops at 51 pages.

### Added — close-day attribution (CRM-T151, CRM-R85)

`rate_basis` on `GET /api/attribution`; `rate_basis` in its answer.

### Added — close-day report conversion (CRM-T149, CRM-R83)

`rate_basis` on the report spec; `MAX_DAY_CELLS`; `rules::fx::convert_rows_dated`. The bridge routes are listed as bodyless in the OpenAPI body test.

### Added — automatic closing (CRM-T148, CRM-R82)

`auto_close_days` on `POST /api/sla-policies`; `new_auto_closed` in the answer of `POST /api/sla/sweep`.

### Added — suggested articles (CRM-T133, CRM-R66)

`rules::suggest` (word overlap, headline words 2, text words 1); `GET /api/tickets/{pid}/suggested-articles`; 3 unit tests and 1 request test.

### Added — log retention (CRM-T140, CRM-R81)

`CRM_EVENT_RETENTION_DAYS`, `CRM_MAIL_RETENTION_DAYS`; `logs_past_horizon` in `GET /api/retention`; `logs_deleted` in the sweep's answer.

### Changed — single-use link tokens (CRM-T146, CRM-R79)

Migration `000050` (`link_tokens`); the `link_tokens` module; the signed confirm and preference tokens are gone from `unsubscribe.rs`.

### Added — small-cell suppression (CRM-T145, CRM-R78)

`CRM_MIN_CELL_SIZE`; `min_cell_size` and `suppressed` in a report's answer and audit entry; `rules::reports::suppress`.

### Added — the inventory test (CRM-T141)

`tests/requests/inventory.rs` fails when a table is not in `spec/compliance/personal-data-inventory.md`, and when the `swept` tables are not those the retention sweep deletes from.

### Added — bridges (CRM-T139, CRM-R72, CRM-R73)

`/api/bridges/ppm/deals/{pid}` and `/api/bridges/wpm/teams-import`, admin only; migration `000049`; `CRM_PPM_SERVICE_URL`, `CRM_WORKER_SERVICE_URL`; `put_team` shared with the team routes.

### Added — clawbacks (CRM-T138, CRM-R71)

`/api/quotas/clawbacks/{pid}`, admin only; migration `000048`; `credited_deals` takes the clawback off and drops a deal clawed back whole.

### Added — `rate_basis=close_day` on the CLV (CRM-T137, CRM-R70)

Each won deal converted at the rate in force on the day it closed; `rate_basis` in the answer, `rate_date` `null`, the distinct rates used and the deals with no rate listed.

### Added — wait-limit actions (CRM-T136, CRM-R69)

`wait_limit_action` and `escalate_to` on `POST /api/sla-policies`; migration `000047`; `new_wait_limit_resolved` and `new_wait_limit_escalated` in the sweep's answer.

### Added — territories of named accounts (CRM-T135, CRM-R68)

`GET`, `PUT` and `DELETE /api/quotas/territories` and `GET /api/quotas/territory-attainment`, admin only; migration `000046`. An account is in at most one territory; revenue is per currency, never converted.

### Added — teams and their quota roll-up (CRM-T134, CRM-R67)

`GET`, `PUT` and `DELETE /api/quotas/teams` and `GET /api/quotas/team-attainment`, admin only; migration `000045`. Currencies are added per currency, never converted.

### Added — deal splits for quotas (CRM-T132, CRM-R65)

Migration `000044` (`deal_splits`); `rules::quota::split`; the three routes; `credited_deals` behind attainment; 1 unit test, 1 request test and the enforcement test extended. Also two clippy fixes in `dashboards.rs`.

### Added — a wait limit on tickets (CRM-T131, CRM-R64)

Migration `000043` (`sla_policies.max_wait_minutes`, `tickets.wait_limit_exceeded_at`); `sweep_waiting`; `new_wait_limits_exceeded` in the sweep answer; the `ticket_wait_limit_exceeded` event; 1 request test.

### Added — customer reply on a ticket (CRM-T130, CRM-R63)

`POST /api/tickets/{pid}/customer-reply`; the status change is now one `transition` shared with `/status`, and the audit entry names `via: customer_reply`; 1 request test.

### Added — CLV margin and look-ahead converted (CRM-T129, CRM-R62)

`margin_converted_minor` and `projection.converted` (`ClvProjectionConverted`) on the CLV read with `convert_to`; 1 request test.

### Added — retention from history on CLV (CRM-T128, CRM-R61)

`retention_from=history` on `GET /api/accounts/{pid}/clv`; `rules::analytics::retention_from_history`; `projection.retention_history_bps`, and `retention_bps` null when read from history; 1 unit test and 1 request test.

### Added — margin and a discounted look-ahead on CLV (CRM-T126, CRM-R60)

`analytics::margin` and `analytics::projected` (integer, half to even); the CLV read takes `margin_bps`, `retention_bps`, `years` and `discount_bps`.

### Added — quotas and commission plans (CRM-T125, CRM-R59)

Migration `000042` (`commission_plans`, `quotas`); `rules::quota` (marginal tiers in whole minor units and basis points, no floating point); `controllers::quotas`; `/api/quotas` and `/api/commission-plans` join the admin-only prefixes.

### Added — opens and clicks as attribution touches (CRM-T124, CRM-R58)

Migration `000041` (`email_events` accepts `opened` and `clicked`); the Brevo, Mailjet, Mailgun, `SendGrid` and SES parsers and `POST /webhooks/email` read them; `apply_event` records them only while consent for the campaign's purpose is granted; `GET /api/attribution` takes `touches` and names it in the answer.

### Added — a public help centre and a deflection report (CRM-T123, CRM-R57)

`CRM_PUBLIC_KB` and `CRM_PUBLIC_KB_CLIENT_LIMIT`; `GET /kb/articles`, `GET /kb/articles/{pid}`, `POST /kb/articles/{pid}/feedback` (public paths, `404` while off); migration `000040` (`kb_daily`, counts only); `GET /api/insights/kb-deflection`.

### Added — time-decay and position-based attribution (CRM-T122, CRM-R56)

`rules::attribution::credit` takes the close time and shares by integer weights through one apportioning rule; `model=time_decay` and `model=position` are accepted. `first`, `last` and `linear` are unchanged.

### Added — SLA clocks that stop while a ticket waits (CRM-T121, CRM-R55)

Migration `000039` (`tickets.paused_at` and the two paused-seconds columns); `rules::sla` and `rules::calendar` gain the arithmetic; the status change, the priority change, the list, the sweep, the SLA dashboard and the support insights use it.

### Added — a sweep for money that skips masking (CRM-T120)

`no_read_a_masked_caller_may_make_returns_the_seeded_amount` walks every `GET` path of the OpenAPI document as the service and as a masked reader.

### Fixed — CLV is masked (CRM-T119)

`clv_minor` and `converted_minor` are `null` for a masked caller (the schema marks `clv_minor` nullable); the count of won deals stays.

### Added — converted CLV (CRM-T118, CRM-R54)

`GET /api/accounts/{pid}/clv` takes `convert_to` and `rate_date`; the answer adds `converted_minor`, `rates` and `unconverted` beside the unchanged per-currency CLV.

### Added — converted forecast (CRM-T117, CRM-R53)

`GET /api/forecast` takes `convert_to` and `rate_date`; the answer adds `converted_minor`, `rates` and `unconverted` beside the unchanged per-currency totals.

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
