# Worldwide by design

*Added 2026-10-30: the worldwide equity audit (CRM-T164). The requirement and the decision below are proposed for the product owner.*

The CRM's target is the whole world, not the United States or the United Kingdom ([scope](scope.md#worldwide-by-design),
[purpose](purpose.md), the opening of [requirements](requirements.md)). Much of the regulatory text, the compliance documents, the seed data
and the defaults were written with UK and EU examples (the UK GDPR, the EU GDPR, PECR, the NHS DTAC, the MHRA, a pound sterling default).
Those are **examples and the first worked cases, not the design.** This file states the rule, audits the spec and the code against it, and
files each gap as a task. It follows the same method and format as the two sibling applications' audits: the workforce planning app's
`spec/worldwide.md` (WPM-R214, WPM-D143, its spec audit WPM-T367 and its locale and convention check WPM-T371) and the project portfolio
app's `governance/worldwide-equity-audit.md` (WW-1, against its `spec/02-scope.md` §2.0).

## Requirement

- **CRM-R95** *(proposed, needs product-owner confirmation)* Every standard, protocol, convention, locale, format and regulation the CRM
  adopts must have **equitable capability for countries, regions and federations worldwide**. No jurisdiction is the default, the
  first-class case or the only case that works: a deployer in any country, in a federation of states, or in a region spanning several
  countries can configure and run the CRM without a code change and without a lesser experience than a UK or US deployer. In particular:
  consent and e-marketing rules, money, names, phone numbers, dates, time zones, working weeks, reporting periods, scripts and their
  direction, plural rules and the language of what a data subject reads. Where coverage is uneven today, that is a gap recorded here and in
  [tasks.md](tasks.md), not a design.

## Decision

- **CRM-D19 Jurisdiction is data, never code; a standard is adopted as a profile, never as the only shape.**
  *Proposed, needs product-owner confirmation* (drafted 2026-10-30 by the AI assistant from the instruction that ordered this audit; it
  mirrors WPM-D143 and the project portfolio app's §2.0 so the three apps read alike).
  - **Jurisdictions are configuration.** A rule that comes from a law (who may be emailed and on what basis, how long consent lasts, what a
    message must carry, retention, the subject-access and erasure duties) takes the jurisdiction's parameters as an input and names no
    country. A deployment-wide setting that stands in for one is labelled with whose value it is. UK and EU values may ship as worked
    example profiles beside at least one profile of a different shape.
  - **Federations and regions are first-class.** A country is not the only unit: a federal state or province, a supranational region (for
    example the EU) and a bloc with shared law are each representable, and a contact or a send may be under more than one layer.
  - **No Anglophone or Western default shapes.** Names (one display name; given and family names optional and never derived from it),
    phone numbers (any script's digits; no one country's trunk rule), dates and calendars (week start and weekend days vary; more than
    Gregorian), time zones (an organization's or user's, not UTC), reporting periods (a fiscal year need not start in January),
    numbers and currency (the minor-unit exponent per ISO 4217, not always 2; no built-in currency) are per-locale or per-jurisdiction.
  - **Standards: international first, national as profiles.** Prefer an international or open standard (ISO 4217, ISO 3166, IANA time
    zones, BCP 47, Unicode CLDR, IETF RFCs) where one exists, and add national or regional ones as profiles of it. Where the only standard
    read is national, say so and record what a deployer elsewhere does instead.
  - **Locales are equal.** A locale is complete or it is marked incomplete; no locale gets features another lacks. Right-to-left, non-Latin
    scripts and plural rules beyond English are supported; text a data subject reads (confirmation, preference and unsubscribe pages,
    template fallbacks) is translated like the interface.
  - **Claims are scoped.** Where the spec says what a law requires, it names the jurisdiction. No sentence says "the law", "statutory" or
    "lawful" without one. A jurisdiction not yet read is recorded as *not yet read*, never assumed to match the UK or the EU.
  - **Equity is tested.** A change that adds a jurisdiction-specific rule is not done without a test that runs it with a second,
    differently shaped profile; a money change is not done without a JPY (exponent 0) and a KWD (exponent 3) case.

## Tasks

- [x] (2026-10-30) CRM-T164 **Worldwide equity audit.** *(CRM-R95, CRM-D19)* Done: the spec audit (19 rows) and the code and UI check
  (45 rows) below, 64 findings in all: 16 right, 23 partial, 25 gaps. The gaps are CRM-T165 to CRM-T176 in [tasks.md](tasks.md).
  Wording fixed in this task, and only where the change is wording (no behaviour): [regulatory.md](regulatory.md) (the tracking-pixel
  sentence and the production list name their regimes and say the rest are not yet read), [index.md](index.md) (the regulatory row),
  [compliance/email-provider-brief.md](compliance/email-provider-brief.md) (three rows and the transfer question),
  [compliance/dpia.md](compliance/dpia.md) and [compliance/dpia-template.md](compliance/dpia-template.md) (the transfer line). **Not done:**
  see "Not audited" at the end.
- The open tasks are listed in [tasks.md](tasks.md) under "Worldwide equity", each with the rows it closes.

## Spec audit (CRM-T164, 2026-10-30)

Method: a search of `spec/` (the role briefs in `locales/` and the copied guides `locales-for-global-sharing-with-svelte/`,
`locales-x-for-roles/` and `special-files-for-public-repos/` left out) for UK, EU and US terms (UK, EU, EEA, GDPR, PECR, CAN-SPAM, CASL,
CCPA, ICO, NHS, DTAC, MHRA, Great Britain, "the law", statutory, lawful, legally, £, GBP, USD, postcode, VAT, tax, fiscal, quarter, weekend,
week start, time zone, first and last name), then a read of each hit and of the requirements for consent, money, names, calendars and
periods. Paths are relative to `spec/`; line numbers are as of commit `b1404e0`. **No sentence in `spec/` says "the law" without a
jurisdiction**; the unscoped claims are "lawful basis", "PECR" or "UK/EEA" used as if they were the only regime.

| # | Where | What it assumes | Verdict | Task |
| --- | --- | --- | --- | --- |
| 1 | `scope.md:3-24`, `purpose.md` (Goals, last row), `requirements.md:3` | The worldwide rule: no country's convention is the default; regimes are configurable named sets; GDPR, UK GDPR and CCPA are examples | **Right**: the rule exists. CRM-R95 and CRM-D19 make it a numbered requirement and decision like WPM-R214 and WPM-D143 | — |
| 2 | `regulatory.md:29-37` | Production needs "UK/EU GDPR + PECR (e-privacy) compliance for the real send path", soft opt-in rules, sender identification | **Gap**: the register names the UK and the EU only; no other region (the US CAN-SPAM Act and state laws, Canada's CASL and PIPEDA, Brazil, India, China, Japan, Australia, South Africa, the Gulf) is listed or marked *not yet read*. **Wording fixed 2026-10-30**: the sentence names the regimes read and says the rest are not yet read; the register by region is the task | T165 |
| 3 | `regulatory.md:19-25` | "A tracking pixel is a PECR/GDPR matter of its own" | **Partial**: scoped, but two regimes as if they were the whole. **Wording fixed 2026-10-30** (the UK and the EU named, other regions not yet read) | T165 |
| 4 | `regulatory.md:38-45`, `compliance/risk-management.md:13-16,92`, `requirements.md:1334` | Medical-device status is "a question for the MHRA" in Great Britain; ISO 14971 | **Partial**: scoped to Great Britain by name, and ISO 14971 is international; no other regulator (for example the US FDA, the EU MDR, Australia's TGA) is named or marked not yet read | T165 |
| 5 | `compliance/control-mapping.md:11-68` | The privacy rows map to GDPR articles only (6, 7, 15, 17, 28, 30, 32, 33, 35, Ch. V) beside NIST, HIPAA and ISO for security | **Gap**: no other data-protection or e-marketing regime has a column or a row; a deployer outside the UK and EU has no mapping | T165 |
| 6 | `compliance/dpia.md:3,77`, `compliance/dpia-template.md:3-4,25`, `compliance/record-of-processing.md:1,9` | The DPIA and the record of processing are GDPR instruments (Art. 35, Art. 30); "transfers outside the UK/EEA"; an Art. 27 representative "if outside the UK/EEA" | **Partial**: correctly named as GDPR documents, but no equivalent (a privacy impact assessment under another regime) is offered. **Wording fixed 2026-10-30** for the transfer lines (the deployer's jurisdiction, the UK/EEA as the GDPR case); the Art. 27 line is GDPR-specific and was left | T165 |
| 7 | `compliance/email-provider-brief.md:107,111,122-123` | Unsubscribe and sender identification are justified by "PECR"; the transfer question asks about "out of the UK or EEA" | **Partial**. **Wording fixed 2026-10-30** (PECR named as the UK's, other regimes not yet read; the transfer question asks about the deployer's jurisdiction). The postal address is code row 4 | T165, T166 |
| 8 | `compliance/gds-api-review.md:1`, `compliance/dtac-supporting.md`, `../docs/united-kingdom/` | UK government API standards; the NHS DTAC form; the only assurance folder is a UK one | **Partial**: named as UK on purpose, but it is the only assurance mapping; no generic one (ISO/IEC 27001, SOC 2) and no other country's scheme, and the `docs/<country>/` convention is not stated | T165 |
| 9 | `tasks.md:153` (CRM-G2) | The production gate for the real send path is a "GDPR/PECR review" | **Partial**: a deployer sending to Canada or the US would pass the gate without the review that applies there. The gate's name was not changed here (it is the product owner's) | T165 |
| 10 | `plan.md:14-15` | Readiness for medical and governmental buyers is judged against HIPAA, NHS DSPT, GDPR, FedRAMP, UK Cyber Essentials Plus and GovAssure | **Partial**: named, but US, UK and EU schemes only | T165 |
| 11 | `requirements.md:959,962` (CRM-R50) | "As everywhere in the CRM, a minor unit is taken as a hundredth"; "not built: currencies whose minor unit is not a hundredth" | **Gap**, recorded honestly in the requirement: JPY (0), KWD and BHD (3) and CLF (4) are wrong wherever an amount is converted, shown or typed (code rows 8 to 11) | T167 |
| 12 | `requirements.md:79-90` (CRM-R6), `requirements.md:472-512` (CRM-R29) | Only `granted` allows a send, for every contact; four purposes fixed in code; no consent per channel; no expiry | **Partial**: the strictest model applied everywhere is safe, but no jurisdiction's own shape can be expressed: an opt-out regime, a soft opt-in for existing customers (the UK's PECR), implied consent that lapses after a period (Canada's CASL, not yet read), or a channel-specific rule | T166 |
| 13 | `requirements.md:892-901` (CRM-R45) | A business calendar has an IANA zone, windows on any weekday (`mon` to `sun`) and a holiday list | **Right**: no weekend, week start or holiday set is assumed. No holiday packs ship (a limit, not a gap: dates are data) | — |
| 14 | `requirements.md:1053-1064` (CRM-R59) | A quota's period is any span of up to 366 days | **Right**: no fiscal-year start is assumed for quotas | — |
| 15 | `requirements.md:779-803` (CRM-R37, CRM-R38) | One display name; the family name is never derived; the `given_name` token falls back to the first word of the display name | **Partial**: the family name rule is right; the given-name fallback is wrong for family-name-first orders ("Wang Fang" gives "Dear Wang") and for names without spaces | T169 |
| 16 | `requirements.md:69-77` (CRM-R5), the reports' `*_month` groups | A month-end forecast snapshot; reports group by calendar month | **Gap**: Gregorian months in the database's zone; no fiscal year or quarter, no organization time zone | T171 |
| 17 | `locale/index.md`, `../contact-relationship-management-ui-with-svelte/spec/translation-register.md:20-28` | Eight UI locales, seven of them drafts written by Claude; the role briefs in four non-English languages, also unreviewed | **Partial**: honestly marked, and a test keeps the register true, but no locale other than English has been read by a native speaker | T175 |
| 18 | `scope.md:56-66` | Quoting, contracts, invoicing and payments are out of scope | **Right**: there is no tax rule in the CRM (the only use of the word is the "like tax bands" analogy in CRM-R59), so no tax regime is assumed | — |
| 19 | `index.md:65` | The regulatory row reads "GDPR / e-privacy (marketing consent) posture" | **Partial**. **Wording fixed 2026-10-30** (data-protection and e-privacy, by region) | T165 |

## Code and UI check (CRM-T164, 2026-10-30)

Method: a read of the code, not the spec. The service is `contact-relationship-management-service-with-rust` (`src/rules/`,
`src/controllers/`, `src/mail.rs`, `src/search.rs`, `src/tasks/seed.rs`, `migration/src/`), the matcher is
`contact-relationship-management-matcher-with-rust/src/`, and the UI is `contact-relationship-management-ui-with-svelte` (`src/lib/`,
`src/routes/`, `src/app.css`, `tests/e2e/`). Paths are relative to the subproject named first (API, matcher or UI). Line numbers are as of
commit `b1404e0`, from search output with the surrounding lines read. **No behaviour was changed by this check.** Candidate wording in the
code (API error text "such as GBP", the "GDPR subject-access export" summary, English template fallbacks) is visible behaviour that a test
or a translation depends on, so it is left to its task.

| # | Area | Where | What the code does | Verdict | Task |
| --- | --- | --- | --- | --- | --- |
| 1 | Consent | API `src/rules/tokens.rs:21`, `src/consents.rs:87,211`, `src/rules/segment.rs` | Four fixed purposes; only `granted` allows a send on every path; no jurisdiction on a contact or a send | **Partial**: the strictest rule for everyone; no regime can be configured (spec row 12) | T166 |
| 2 | Consent | API `src/controllers/marketing/consent.rs:200,202` | Confirmation links last 7 days and preference links 90 days, constants | **Right**: not claimed as any law's value | — |
| 3 | Suppression | API `src/models/deliveries.rs`, `src/provider.rs` | A per-contact suppression list fed by bounces and complaints; unsubscribes suppress through consent | **Right**: no country's rule is assumed | — |
| 4 | Message content | API `src/mail.rs:34-60,287` | Each message carries `List-Unsubscribe` and `List-Unsubscribe-Post` (RFC 2369, RFC 8058) and an unsubscribe link; no sender identification and no postal address | **Gap**: several regimes ask for a postal or contact address in each commercial message (the UK's PECR as the brief records it; the US CAN-SPAM Act and Canada's CASL, not yet read here); there is no setting to carry one | T166 |
| 5 | Public pages | API `src/controllers/marketing/public.rs:15,120` | The confirmation, preference and unsubscribe pages are English with `lang="en"` (CRM-R29: "not built: translating the public pages") | **Gap**: what a data subject in any country reads about their own consent is English only | T175 |
| 6 | Retention | API `src/rules/privacy.rs:25-38` | `CRM_RETENTION_DAYS` defaults to 365 with a floor of 30, one value per deployment; the basis is not named | **Partial**: configurable, but one deployment-wide value and no named source | T165 |
| 7 | Subject rights | API `src/openapi_operations.rs:1229` | The access export is described as "GDPR subject-access export" | **Partial**: the route serves any regime's access right; the text names one | T165 |
| 8 | Money | UI `src/lib/api/crm.ts:145-155` | `money()` formats with `Intl.NumberFormat(locale, { style: "currency", currency })`, but of `minor / 100` | **Gap**: a JPY amount shows at a hundredth of its value, a KWD or BHD amount at ten times. The harmonized fix is the behaviour of the sibling apps' shared `src/lib/money.ts` (project portfolio commit `956d9ed`, workforce planning WPM-T373): the exponent from `Intl.NumberFormat#resolvedOptions().maximumFractionDigits`, one formatter and parser, `toMinor`, `parseAmount`, tests with JPY and KWD | T167 |
| 9 | Money | UI `src/routes/attribution/+page.svelte:62-64`, `src/routes/reports/+page.svelte:154` | Two more hand-made formatters, each `minor / 100` | **Gap**, as row 8 | T167 |
| 10 | Money input | UI `src/routes/deals/+page.svelte:96,103,150`, `src/routes/deals/[pid]/+page.svelte:91`, `src/routes/quotas/+page.svelte:126,190` | `Math.round(Number(x) * 100)` to store and `amount_minor / 100` to edit; `Number()` refuses a decimal comma ("12,50") | **Gap**: a yen deal typed as 1500 is stored as 150,000 minor units; German, French, Spanish and other users cannot type a decimal | T167 |
| 11 | Money conversion | API `src/rules/fx.rs:153-165` | `convert` multiplies minor units by the rate, so both currencies are assumed to have the same exponent | **Gap**: USD 1.00 (100 minor) at 150 JPY becomes 15,000 minor yen, ¥15,000 instead of ¥150; every converted report, forecast, attribution, ROI and CLV figure between currencies of different exponents is wrong | T167 |
| 12 | Money arithmetic | API `src/rules/analytics.rs:20-35`, `src/rules/quota.rs:102` | Integer minor units per currency; currencies never summed; commission in integer arithmetic | **Right**: no exponent is needed here | — |
| 13 | Currency codes | API `src/rules/fx.rs:23-27`; `src/controllers/sales.rs:587`, `src/controllers/marketing/campaign.rs:63`; `migration/src/m20220101_000002_sales.rs:79`, `m20220101_000003_marketing.rs:41` | Rates, quotas and `convert_to` take any three capital letters; a deal's and a campaign's currency is any non-empty text, `VARCHAR` with no check | **Partial**: not checked against ISO 4217, so an unknown code (or "pounds") is stored and later cannot be formatted | T168 |
| 14 | Default currency | API `src/controllers/marketing/campaign.rs:24,50-51` (OpenAPI `@default GBP`); UI `src/routes/quotas/+page.svelte:48`, `src/routes/fx-rates/+page.svelte:12-13`, `src/routes/deals/+page.svelte:96` (`?? "GBP"`) | A campaign without a currency is GBP; the quota form starts at GBP; the rate form at USD to GBP; a deal row with no currency is shown in GBP | **Gap**: a built-in pound sterling default (the sibling apps removed theirs: project portfolio WW-1.3, workforce planning WPM-T376) | T168 |
| 15 | Currency text | API `src/rules/fx.rs:27`; UI `src/lib/server/demo.ts:3086` | "a currency is three capital letters, such as GBP" | **Partial**: an example only, but always the same one | T168 |
| 16 | Names | API `src/rules/templates.rs:27-29,309-313`, `src/controllers/templates.rs:367` | The `given_name` token, without a stored given name, is the first word of the display name | **Gap**: wrong for family-name-first orders and names without spaces (spec row 15) | T169 |
| 17 | Template text | API `src/rules/templates.rs:29-54` | Token fallbacks are English, fixed in code: "there", "customer", "your organisation", "the team", "your role" | **Gap**: a template written in Arabic or Hindi fills with English words unless each template sets its own fallback | T169 |
| 18 | Names (matcher) | matcher `src/normalize.rs:40-80`, `src/matcher.rs:294-313` | The display name is split into given = first word and family = the rest (a particle starts the family name; "Family, Given" is read); a swapped order is tried and kept if it agrees better | **Partial**: the swap helps family-name-first orders; names written without spaces (Chinese, Japanese, Thai) cannot be split | T169 |
| 19 | Names (matcher) | matcher `src/normalize.rs:26-27,192`, `src/nicknames.rs:1` | Honorifics and suffixes are English (`mr`, `dame`, `esq`); nicknames are "English given-name nicknames"; legal-form words are mostly English and Western European (`ltd`, `gmbh`, `srl`, `pty`) | **Partial**: other languages' honorifics, nicknames and legal forms (for example `kk`, `pvt`, `sdn bhd`, `ooo`, `sa de cv`) are not removed, so they lower a match score | T169 |
| 20 | Imports | API `src/rules/record_imports.rs:78-95` | Column guesses are English header names ("first name", "surname", "company") | **Partial**: the mapping can be set by hand; only English headers are guessed | T169 |
| 21 | Names | API contact entity; `requirements.md` CRM-R37 | One `display_name`, optional `given_name` and `family_name`, no title, no first/last field | **Right** | — |
| 22 | Phone, address | API `src/rules/tokens.rs:9` (channels `email`, `phone`, `post`, `none`) | The CRM stores no phone number or address: they belong to the person and organization registries | **Right**: nothing here assumes a shape (the registries are not audited) | — |
| 23 | Phone (matcher) | matcher `src/config.rs:61,82` | `default_calling_code` is unset by default | **Right**: no country's code is the default | — |
| 24 | Phone (matcher) | matcher `src/normalize.rs:150-180` | Only ASCII digits are kept; with a default calling code a leading `0` is dropped as a trunk prefix (and `1` gets the NANP ten-digit rule) | **Partial**: a number in Arabic-Indic or Devanagari digits is read as no number; dropping the `0` is wrong where it is part of the number (Italy) | T170 |
| 25 | Search | API `src/search.rs:15-19`, `src/rules/search.rs:13-21` | Words are runs of Unicode letters and digits; PostgreSQL's `simple` configuration with `unaccent` | **Partial**: neutral for scripts that separate words with spaces; Chinese, Japanese and Thai text is one "word", so a part of a name is not found | T170 |
| 26 | Business hours | API `src/rules/calendar.rs:33-58,100-108` | Windows per weekday `mon` to `sun`, an IANA zone (summer time followed), explicit holiday dates | **Right**: any weekend, any working week, any holiday set | — |
| 27 | Today (API) | API `src/controllers/quotas.rs:969`, `src/controllers/sales.rs:977` | "Today" for attainment and the forecast snapshot day is the UTC date | **Gap**: the wrong day near midnight far from UTC; there is no organization time zone | T171 |
| 28 | Periods | API `src/rules/reports.rs:130-215` | `date_trunc('month', …)` in the database session's zone; months only | **Gap**: no fiscal year or quarter, no zone chosen by the organization (spec row 16) | T171 |
| 29 | Today (UI) | UI `src/routes/quotas/+page.svelte:40`, `src/routes/followups/+page.svelte:47`, `src/lib/components/CampaignTimeline.svelte:21`, `src/routes/leads/import/+page.svelte:26`, `src/routes/fx-rates/+page.svelte:15`, `src/lib/components/RecordImport.svelte:48` | `new Date().toISOString().slice(0, 10)`: the UTC date | **Gap**, as row 27 | T171 |
| 30 | Dates (UI) | UI `src/routes/notifications/+page.svelte:37`, `CampaignTimeline.svelte:63`, `src/routes/followups/+page.svelte:54`, `src/routes/contacts/[pid]/+page.svelte:458` | `Intl.DateTimeFormat(i18n.locale, …)` | **Right** | — |
| 31 | Dates (UI) | UI `src/routes/tickets/+page.svelte:56`, `src/routes/contacts/[pid]/+page.svelte:274` | `toLocaleString()` with no locale: the browser's, not the page's | **Gap** | T172 |
| 32 | Weekend (UI) | UI `src/routes/followups/+page.svelte:161` | `weekendDays={[0, 6]}`: Saturday and Sunday in every locale | **Gap**: a Friday and Saturday (or Friday only) weekend is shown as working days | T172 |
| 33 | Week start (UI) | UI `src/routes/followups/+page.svelte:158` (`locale={i18n.locale}` to the calendar) | The calendar is given the page's locale; whether its first day follows the locale, and what an `-001` locale resolves to, was not checked | **Partial** | T172 |
| 34 | Percent (UI) | UI `src/lib/format.ts:11,33`, `src/routes/articles/+page.svelte:24`, `src/lib/components/ClvPanel.svelte:102`, `src/routes/quotas/+page.svelte:611`, `src/routes/leads/import/+page.svelte:96`, `src/lib/components/RecordImport.svelte:119` | Percentages built by hand (`Math.round(x * 100)` and a `%`) | **Gap**: the sign's place, spacing and digits differ by locale (French "50 %", Arabic and Hindi digits) | T172 |
| 35 | Numbers in (UI) | UI `src/lib/components/ClvPanel.svelte:29`, `src/routes/quotas/+page.svelte:70` | Percent inputs read with `Number()` | **Gap**: a decimal comma is refused | T172 |
| 36 | Plurals | UI `src/lib/i18n.svelte.ts:181-190`, `src/lib/components/Board.svelte:86,104`, `src/lib/locales/ar-001.ts:8,66` | No `Intl.PluralRules`; the board picks "one" or "other" by `count === 1`, the English rule, in every locale; 10 strings in `en-001` fill a `{count}` with one form, and Arabic has "{count} أيام" for every number | **Gap**: Arabic (six forms), Welsh (six) and others are wrong for most counts | T173 |
| 37 | Right-to-left | UI `src/lib/i18n.svelte.ts:56`, `src/routes/+layout.svelte:75`, `tests/e2e/smoke.spec.ts:370` | `dir="rtl"` for `ar-001`, tested end to end | **Right** | — |
| 38 | Right-to-left | UI `src/app.css:135,346-370` | `text-align: left` on every table cell and seven `border-left` declarations (14 logical declarations elsewhere) | **Gap**: tables and state cards do not mirror in Arabic | T174 |
| 39 | Right-to-left | UI `src/routes/leads/[pid]/+page.svelte:17`, `src/routes/deals/[pid]/+page.svelte:113`, `src/routes/accounts/[pid]/+page.svelte:54`, `src/routes/help/[pid]/+page.svelte:46`, `src/routes/tickets/+page.svelte:114`, `src/routes/campaigns/+page.svelte:111`, `src/routes/quotas/+page.svelte:611` | Literal "←" and "→" for back links, moves, funnels and tiers | **Gap**: they point the wrong way in Arabic | T174 |
| 40 | Locales | UI `src/lib/i18n.svelte.ts:21-30`, `spec/translation-register.md:20-28` | Eight locales; six are `-001` (world), but German is `de-de` and Chinese `zh-cn`, so an Austrian or Swiss reader is labelled "Deutschland" and Traditional Chinese readers get Simplified; seven of eight are unreviewed drafts | **Partial** | T175 |
| 41 | Locales | UI `tests/unit/translation-register.test.ts` | Every locale holds every key and the same placeholders as English | **Right** | — |
| 42 | Locales | UI `src/routes/deals/+page.svelte:142,160-162` and seven more files (57 `header: "…"` literals in 8 files), `src/routes/partners/+page.svelte:105` | Data-grid column headers and a matrix label are English literals outside the catalogue | **Gap**: every non-English locale shows English headers | T175 |
| 43 | Seed and demo | API `src/tasks/seed.rs:117`; UI `src/lib/server/demo.ts:315-324,498-508,843-845,1127,1284` | Every seeded deal is GBP; the tour's helpers take `pounds` and multiply by 100; the rates are from GBP | **Gap**: the demo has one currency (exponent 2), so no exponent bug can show in it | T176 |
| 44 | Holidays | API `src/rules/calendar.rs:100-150` | Holidays are a list of up to 400 dates per calendar; no pack ships | **Right** in shape (any calendar's holidays can be entered as dates); no pack is a limit | — |
| 45 | Tax | API, UI (searched for `tax`, `vat`) | No tax rule anywhere | **Right**: out of scope (spec row 18) | — |

### What the audit found, in short

- **64 findings: 16 right, 23 partial, 25 gaps** (spec: 4 right, 11 partial, 4 gaps; code and UI: 12 right, 12 partial, 21 gaps).
- **The five worst gaps:**
  1. **Money assumes two decimals end to end** (code rows 8 to 11, spec row 11): the UI shows and stores `minor / 100` and `x * 100`,
     and the service's exchange-rate conversion ignores the exponent, so JPY, KWD and BHD amounts and every conversion between currencies
     of different exponents are wrong. CRM-T167; the fix follows the sibling apps' `money.ts`.
  2. **The regulatory register and the compliance mapping are UK and EU only** (spec rows 2, 5, 9): no other region is listed or marked
     not yet read, and the production gate CRM-G2 is a GDPR/PECR review. CRM-T165.
  3. **Consent has one shape for every jurisdiction and messages carry no postal address** (spec row 12, code rows 1, 4): no opt-out,
     soft opt-in or expiring implied consent can be expressed, and the address several regimes ask for cannot be sent. CRM-T166.
  4. **A built-in pound sterling default** (code row 14): campaigns, the quota form, the rate form and a deal row without a currency.
     CRM-T168.
  5. **"Today" and report months are UTC, and there is no fiscal period** (spec row 16, code rows 27 to 29): the wrong day far from UTC,
     and a fiscal year that does not start in January cannot be reported. CRM-T171.
- **Also serious:** the English plural rule in every locale (code row 36), English text on the pages a data subject reads (row 5) and
  in template fallbacks (row 17), physical CSS and arrows in Arabic (rows 38, 39), and 57 English grid headers (row 42).
- **What is already right:** the worldwide rule in the scope, business calendars with any working week, zone and holiday dates, quota
  periods of any span, integer minor units that never sum across currencies, one display name with optional parts, no phone or address
  shape in the CRM, no default calling code, `dir="rtl"` for Arabic, locale parity kept by a test, and no tax rule.

## Not audited

- **The identity registries** (person, organization, worker services), which own names, phone numbers and addresses: the CRM holds none of
  them, so their shapes were not read.
- **The role briefs in `locales/`** and the copied guides under `spec/` were left out of the spec search.
- **Every UI string's wording**: only the catalogue structure, placeholders and plurals were checked, not whether each translation is
  right; nothing was read by a native speaker.
- **Hard-coded English beyond the grid headers**: only `header: "…"` literals and the two cases named were counted; a full scan of English
  text in markup outside `t()` was not done.
- **The calendar, kanban and data-grid packages** (`@lilydesignsystem/*`): their week start, mirroring and number formatting were not read.
- **Nothing was run**: no browser, no service, no test; line numbers come from search output with the surrounding lines read.
- **No law of any region was read for this audit**: the regimes named (CAN-SPAM, CASL and others) are named as candidates for the
  register, not as facts about what they require.
- **The MCP tools' text, the OpenAPI descriptions beyond row 7, the CSV exports and the email-provider adapters' payloads** were not read
  for locale or jurisdiction assumptions.
