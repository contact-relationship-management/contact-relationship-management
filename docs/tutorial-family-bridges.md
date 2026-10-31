# Tutorial: the family bridges

The CRM talks to two sibling services: the project portfolio service (PPM) and the
worker service (WPM). This tutorial sets up the bridges, sends the CRM a signed
hire and leave event the way WPM does, and runs the employment check.

> **Not run against a running WPM or PPM.** Every detail here is taken from the
> service's code and request tests (`src/controllers/bridges.rs`,
> `src/rules/wpm_events.rs`, `src/rules/employment_check.rs`, `src/clients.rs`,
> `src/settings.rs`, `tests/requests/wpm_events.rs`,
> `tests/requests/employment_check.rs`). Those tests sign requests with the same
> algorithm WPM uses and answer the employment check from a stand-in WPM. The
> `openssl` signature below was checked against the RFC 4231 test vector the unit
> tests use; the `curl` calls were not run for this tutorial. Use synthetic data
> only.

Paths in this tutorial are relative to
`contact-relationship-management-service-with-rust/`.

## The four bridges

| Route | Direction | What it does | Requirement |
|---|---|---|---|
| `POST /api/bridges/ppm/deals/{pid}` | CRM → PPM | Opens a delivery idea in PPM for a won deal, once per deal | CRM-R72 |
| `POST /api/bridges/wpm/teams-import` | CRM ← WPM | Reads WPM's active workers and sets one CRM team per department; each member is the worker's `person:` reference | CRM-R73 |
| `POST /api/bridges/wpm/events` | WPM → CRM | WPM posts hire and leave events. A hire pauses a contact's marketing; a leave resumes it | CRM-R92, CRM-R93 |
| `POST /api/bridges/wpm/employment-check` | CRM → WPM | Asks WPM which live contacts are its workers today, and marks them as employees | CRM-R94 |

All four are under `/api/bridges`, which is admin only
(`src/auth/policy.rs`, `ADMIN_ONLY_PREFIXES`). The one exception is
`/api/bridges/wpm/events`, which needs no session or bearer token, because the
event's signature authenticates it. The admin rule is enforced only when
`CRM_REQUIRE_AUTH` is on. It is off by default, so the `curl` calls below need no
token in development.

The three outbound bridges start in **stub mode**. Unless
`CRM_UPSTREAM_MODE=http` is set, nothing leaves the process, and the answer has
`"mode": "stub"` and says what would have been sent. The events route does not
depend on the mode.

A CRM user is a person, named `person:<sign-in subject>` (CRM-D18). Owners,
quota holders and team members all use that form. On input, `worker:<id>` is
still accepted and is stored as `person:<id>`.

## Environment variables

The service reads these settings from the environment once, on first use
(`src/settings.rs`), so restart the service after you change one.

| Variable | Used by | Meaning |
|---|---|---|
| `CRM_UPSTREAM_MODE` | PPM, teams import, employment check | `http` (any case) sends real requests. Anything else, or unset, means stub mode |
| `CRM_PPM_SERVICE_URL` | PPM bridge | The PPM base URL. The CRM posts `{title, pitch, tags}` to `{url}/api/ideas` |
| `CRM_PPM_SERVICE_TOKEN` | PPM bridge | Optional bearer token, sent only to PPM (CRM-R87, CRM-T153) |
| `CRM_WORKER_SERVICE_URL` | teams import, employment check | The WPM base URL. The CRM calls `GET {url}/api/workers?status=active&fields=pid,department,person_ref&limit=200&offset=…` (a WPM without `fields`, from before WPM-R216, answers `422` and the import fails) and `POST {url}/api/bridges/overlap` |
| `CRM_WORKER_SERVICE_TOKEN` | teams import, employment check | Bearer token, sent only to WPM. The overlap check needs it to belong to an HR identity of the organization below (CRM-D16) |
| `CRM_WPM_ORGANIZATION_REF` | employment check | The `organization:` URN to ask WPM about. A value that is not an `organization:` URN counts as unset |
| `CRM_WPM_WEBHOOK_SECRET` | events | The shared HMAC secret, at least 32 characters (a shorter value counts as unset). While it is unset, the events route answers `404`. Set the same value as WPM's `WPM_WEBHOOK_SECRET_<NAME>` for the subscription |

How the tokens are sent (`src/clients.rs`, `with_credential`):

- A token goes only over `https`, or over `http` to `localhost` or a loopback
  address. For any other `http` address the call fails and nothing is sent.
- The bridge client follows no redirects and gives up after 10 seconds.
- Without a token, the CRM sends no `Authorization` header. That works only
  against a service with authentication off.

## 1. Start the service with a webhook secret

Use the quick start in the [service README](../contact-relationship-management-service-with-rust/README.md)
for the database. Then:

```bash
cd contact-relationship-management-service-with-rust
export CRM_WPM_WEBHOOK_SECRET='dev-only-wpm-webhook-secret-0123456789'   # 38 characters
cargo run -- start
```

The service listens on `http://localhost:5150` (`config/development.yaml`).

## 2. Create a contact for a synthetic person

The events find contacts by `person_ref`, so create a contact with a known
reference and grant marketing consent. The examples use `jq`.

```bash
CRM=http://localhost:5150
PERSON='person:3f0c2b1e-0000-4000-8000-000000000101'
ORG='organization:3f0c2b1e-0000-4000-8000-0000000000aa'

PID=$(curl -sS -X POST "$CRM/api/contacts" -H 'content-type: application/json' \
  -d "{\"person_ref\":\"$PERSON\",\"display_name\":\"Ada Example\"}" | jq -r .pid)

curl -sS -X POST "$CRM/api/contacts/$PID/consent" -H 'content-type: application/json' \
  -d '{"action":"granted","source":"web form"}'
```

## 3. Send a signed hire event

WPM signs each delivery with one header:

```text
x-wpm-signature: t=<unix seconds>,v1=<hex HMAC-SHA256(secret, "<t>.<raw body>")>
```

- The signed message is the timestamp, a `.` and the raw request body, exactly
  the bytes sent. If the body is re-encoded after signing, the signature fails.
- `t` must be within 300 seconds of the CRM's clock, before or after
  (`TOLERANCE_SECS`).
- The `v1` value is lowercase or uppercase hex, compared in constant time. A
  header with `t` or `v1` missing or repeated fails.
- The body may be at most 16 KiB (`MAX_BODY`).
- WPM also sends `x-wpm-event` and `x-wpm-delivery`. The CRM does not read them.

The body is `{id, kind, occurred_at, subject, data}`:

| Member | Meaning |
|---|---|
| `id` | The event id, 1 to 100 characters, the same on every retry. A repeated id changes nothing |
| `kind` | `worker_hired`, `worker_terminated` and `worker_retired` are acted on. `worker_activated` and `leave_approved` are ignored (`not_used`); any other kind is ignored (`unknown_kind`) |
| `occurred_at` | RFC 3339; required for the three kinds acted on. It orders the events (CRM-R93) |
| `subject` | `worker:<pid>`; not read |
| `data` | `person_ref` (a `person:` URN), `organization_ref` (an `organization:` URN) and `effective_on` (`YYYY-MM-DD`). For the two endings WPM also sends `from` and `to`, which the CRM does not read. If `data` is missing (a WPM from before WPM-R207), the event is ignored (`no_data`) |

Sign and send a hire with `openssl` and `curl`:

```bash
SECRET="$CRM_WPM_WEBHOOK_SECRET"

wpm_send() {   # usage: wpm_send '<json body>'
  local body="$1" t sig
  t=$(date +%s)
  sig=$(printf '%s.%s' "$t" "$body" | openssl dgst -sha256 -hmac "$SECRET" | sed 's/^.*= //')
  curl -sS -X POST "$CRM/api/bridges/wpm/events" \
    -H 'content-type: application/json' \
    -H "x-wpm-signature: t=$t,v1=$sig" \
    --data-binary "$body"
  echo
}

wpm_send "{\"id\":\"evt-0001\",\"kind\":\"worker_hired\",\"occurred_at\":\"2026-10-30T12:00:00Z\",\"subject\":\"worker:3f0c2b1e-0000-4000-8000-0000000000f1\",\"data\":{\"person_ref\":\"$PERSON\",\"organization_ref\":\"$ORG\",\"effective_on\":\"2026-11-02\"}}"
```

Use `printf '%s'`, not `echo`, so that no newline is added to the signed text.
Use `--data-binary`, so that `curl` sends the body exactly as signed. The `sed`
removes the `SHA2-256(stdin)=` prefix and its space that OpenSSL prints (checked with OpenSSL 4.1); LibreSSL
prints the bare hex, which the `sed` leaves alone.

The expected answer:

```json
{"result":"paused","reason":null,"contacts":1}
```

The contact is now an employee of the organization:

```bash
curl -sS "$CRM/api/contacts/$PID" | jq .employments
# [{"organization_ref":"organization:3f0c…00aa","since":"2026-11-02"}]
```

While a contact has any employment, segments leave them out, a campaign skips
them ("an employee, by the worker service: marketing paused"), and a nurture step
waits. Their consent does not change. Each change is audited on the contact as
`employment_recorded` or `employment_ended`, with `via: wpm`.

## 4. Send a leave, a repeat and an out-of-order event

```bash
# The leave: a new id, recorded later than the hire.
wpm_send "{\"id\":\"evt-0002\",\"kind\":\"worker_terminated\",\"occurred_at\":\"2026-10-30T13:00:00Z\",\"subject\":\"worker:3f0c2b1e-0000-4000-8000-0000000000f1\",\"data\":{\"person_ref\":\"$PERSON\",\"organization_ref\":\"$ORG\",\"effective_on\":\"2026-11-30\",\"from\":\"active\",\"to\":\"terminated\"}}"
# {"result":"resumed","reason":null,"contacts":1}

# The same id again (a WPM retry): nothing changes.
wpm_send "{\"id\":\"evt-0002\",\"kind\":\"worker_terminated\",\"occurred_at\":\"2026-10-30T13:00:00Z\",\"subject\":\"worker:3f0c2b1e-0000-4000-8000-0000000000f1\",\"data\":{\"person_ref\":\"$PERSON\",\"organization_ref\":\"$ORG\",\"effective_on\":\"2026-11-30\",\"from\":\"active\",\"to\":\"terminated\"}}"
# {"result":"duplicate","reason":null,"contacts":0}

# A hire recorded before that leave, arriving after it: stale, nothing changes.
wpm_send "{\"id\":\"evt-0003\",\"kind\":\"worker_hired\",\"occurred_at\":\"2026-10-30T12:30:00Z\",\"subject\":\"worker:3f0c2b1e-0000-4000-8000-0000000000f1\",\"data\":{\"person_ref\":\"$PERSON\",\"organization_ref\":\"$ORG\",\"effective_on\":\"2026-11-02\"}}"
# {"result":"stale","reason":null,"contacts":0}
```

A leave ends the employment with that organization only. Marketing resumes when
the contact has no employment left, under the consent the contact already had.

Ordering (CRM-R93): the CRM stores the last hire or leave it applied for each
person and organization. Events are compared by `occurred_at`, then by
`effective_on` when two events share the same second. An older event is
`stale`. An event with the same stamp as the last one is applied, so the event
that arrives later wins. The CRM records this order only while the person is a
contact. The retention sweep (`POST /api/retention/sweep`) prunes a stored
leave once it is older than the event retention horizon
(`CRM_EVENT_RETENTION_DAYS`, default 30 days, at least 7). An event that arrives
after that is not ordered.

### Answers

| Status | When |
|---|---|
| `404` | `CRM_WPM_WEBHOOK_SECRET` is unset or shorter than 32 characters |
| `401` | `x-wpm-signature` is missing, wrong, malformed, or more than 300 seconds old or ahead |
| `400` | The body is not an event (not JSON, no `kind`, or an `id` that is empty or longer than 100 characters) |
| `413` | The body is over 16 KiB |
| `422` | A hire or leave whose `data` is malformed, or whose `occurred_at` is missing or not RFC 3339 |
| `200` | Everything else, with `result`: `paused`, `already_paused`, `resumed`, `not_paused`, `no_contact` (no live contact carries the person), `duplicate`, `stale` or `ignored` (with `reason` `no_data`, `not_used` or `unknown_kind`) |

The CRM answers `200` to events it ignores, so WPM does not retry them.

## 5. The employment check

Events tell the CRM only about hires and leaves that happen after the
subscription. The employment check finds contacts who were already WPM
workers.

In stub mode (the default), it sends nothing and reports what it would ask:

```bash
curl -sS -X POST "$CRM/api/bridges/wpm/employment-check"
# With only the contact above in the database (asked counts the distinct references):
# {"mode":"stub","organization_ref":null,"asked":1,"batches":1,"workers":0,
#  "recorded":0,"already_paused":0,"left_by_event":0}
```

To ask WPM, restart the service with:

```bash
export CRM_UPSTREAM_MODE=http
export CRM_WORKER_SERVICE_URL=https://wpm.example.org          # http is allowed only to this machine
export CRM_WORKER_SERVICE_TOKEN='<token of an HR identity of the organization in WPM>'
export CRM_WPM_ORGANIZATION_REF='organization:3f0c2b1e-0000-4000-8000-0000000000aa'
```

What the CRM sends (`clients::overlap_check`): `POST
{CRM_WORKER_SERVICE_URL}/api/bridges/overlap` with
`Authorization: Bearer $CRM_WORKER_SERVICE_TOKEN` and the body

```json
{"organization_ref": "organization:…", "person_refs": ["person:…", "…"]}
```

The body holds each distinct, well-formed `person:` reference of a live contact
once, in sorted order, at most 500 per call. The erasure tombstone and malformed
references are left out, and no name is sent. WPM answers
`{"organization_ref", "results": [{"person_ref", "worker"}]}`.

What happens to the contacts:

- The CRM reads every answer strictly before it writes anything. One failed
  call, a refusal, or a bad answer is `502` and records nothing. A bad answer is
  one for another organization, with no `results`, with a reference that was not
  asked about, or with a reference answered twice. In `http` mode,
  `CRM_WPM_ORGANIZATION_REF` unset is also `502`.
- For each person WPM says is a worker today, every live contact with that
  reference is marked an employee of the organization from today. This works
  as a hire event does: marketing is paused, consent is unchanged, and the
  change is audited as `employment_recorded` with `via: wpm` and
  `source: employment_check`. The response counts those contacts in `recorded`;
  contacts that were already marked are counted in `already_paused`.
- If the last event applied for that person was a leave, the person is left
  alone and counted in `left_by_event`.
- A "no" changes nothing. The check never ends an employment.
- The check itself is audited once, as `employment_checked` on `bridge`, with
  the counts only and no reference.

The UI has a button for this check on `/quotas`, beside the teams import.

## 6. With authentication on

With `CRM_REQUIRE_AUTH` on, the PPM bridge, the teams import and the employment
check need a bearer token that the ABAC policy allows to perform destructive
actions. Under the built-in policy, that is `access=admin` or a service token.
The events route stays public and is authenticated by its signature alone.

## What is not verified

- None of this has been run against a running WPM or PPM. The contracts were
  read from those services' source (2026-10-28 and 2026-10-30, see the comments
  in `src/clients.rs`), and the tests use stand-ins.
- The CRM uses one HR token held by the deployer and one configured
  organization, rather than the caller's own token as WPM-D135 asks (CRM-D16).
  The product owner confirmed this on 2026-10-31 as the accepted design.
- See [spec/integrations.md](../spec/integrations.md#family-bridges) and
  [spec/tasks.md](../spec/tasks.md) (CRM-T139, CRM-T153, CRM-T158 to CRM-T162)
  for the limits recorded with each change.
