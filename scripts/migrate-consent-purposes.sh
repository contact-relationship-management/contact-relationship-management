#!/usr/bin/env bash
# Migration 21 (consent per purpose, CRM-R29) against a database that already holds data (CRM-T96).
# Builds a database at migration 20 holding N contacts with consent in every state and their consent
# history, plus campaigns and nurture sequences, migrates forward with the data in place, and checks
# that nothing was lost or changed, that every old event is a `marketing` event, and that the
# migration rolls back and forward again.
# Usage: N=500 scripts/migrate-consent-purposes.sh   (needs psql and the debug binary; a Postgres
# reachable with PG_HOST, PG_PORT, PG_USER and PG_PASSWORD, defaulting to localhost:5432 loco/loco)
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
SVC="$(pwd)/contact-relationship-management-service-with-rust"
BIN=$SVC/target/debug/contact-relationship-management
: "${PG_HOST:=localhost}" "${PG_PORT:=5432}" "${PG_USER:=loco}" "${PG_PASSWORD:=loco}"
export PGPASSWORD="$PG_PASSWORD"
DB=crm_mig_consent
export DATABASE_URL="postgres://$PG_USER:$PG_PASSWORD@$PG_HOST:$PG_PORT/$DB"
N=${N:-500}
admin() { psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d postgres -q -c "$1"; }
sql() { psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$DB" -Atc "$1"; }
run() { (cd "$SVC" && "$BIN" "$@" 2>&1); }
fail=0
ok() { echo "  ok    $1"; }
bad() { echo "  FAIL  $1"; fail=1; }
check() { [[ "$2" = "$3" ]] && ok "$1" || bad "$1 (got $2, want $3)"; }
trap 'admin "DROP DATABASE IF EXISTS $DB" >/dev/null 2>&1' EXIT
admin "DROP DATABASE IF EXISTS $DB" >/dev/null 2>&1
admin "CREATE DATABASE $DB" || exit 1
psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$DB" -q \
  -c 'CREATE EXTENSION IF NOT EXISTS "uuid-ossp"; CREATE EXTENSION IF NOT EXISTS citext; CREATE EXTENSION IF NOT EXISTS unaccent;' >/dev/null 2>&1

echo "== 1. migrate to the latest, then return to migration 20 (the schema before purposes)"
run db migrate | tail -1
run db down 1 | tail -1
check "purpose column is gone from consent_events" \
  "$(sql "select count(*) from information_schema.columns where table_name='consent_events' and column_name='purpose'")" 0
check "contact_consents is gone" "$(sql "select to_regclass('contact_consents') is null")" t

echo "== 2. fill it with $N contacts in every consent state, with history, campaigns and sequences"
sql "insert into contacts (pid, person_ref, display_name, status, preferred_channel, marketing_consent, created_at, updated_at)
     select uuid_generate_v4(), 'person:' || uuid_generate_v4(), 'Person ' || i, 'active', 'email',
            (array['granted','withdrawn','never'])[1 + i % 3], now(), now()
     from generate_series(1, $N) i" >/dev/null
sql "insert into consent_events (pid, contact_pid, action, source, occurred_at, created_at, updated_at)
     select uuid_generate_v4(), pid, marketing_consent, 'seed', now(), now(), now() from contacts where marketing_consent <> 'never'" >/dev/null
sql "insert into campaigns (pid, kind, name, status, cost_minor, currency, recipients, delivered, opened, clicked, unsubscribed, created_at, updated_at)
     select uuid_generate_v4(), 'email', 'Campaign ' || i, 'draft', 0, 'GBP', 0, 0, 0, 0, 0, now(), now() from generate_series(1, 20) i" >/dev/null
sql "insert into nurture_sequences (pid, name, created_at, updated_at)
     select uuid_generate_v4(), 'Sequence ' || i, now(), now() from generate_series(1, 5) i" >/dev/null
CONTACTS=$(sql "select count(*) from contacts"); EVENTS=$(sql "select count(*) from consent_events")
CONSENT_HASH=$(sql "select md5(string_agg(pid::text || marketing_consent, '|' order by pid)) from contacts")
EVENT_HASH=$(sql "select md5(string_agg(pid::text || action || source, '|' order by pid)) from consent_events")
echo "  contacts: $CONTACTS  consent_events: $EVENTS  campaigns: $(sql 'select count(*) from campaigns')  sequences: $(sql 'select count(*) from nurture_sequences')"

echo "== 3. migrate up with the data in place"
run db migrate | tail -2
check "no contact lost" "$(sql 'select count(*) from contacts')" "$CONTACTS"
check "no consent event lost" "$(sql 'select count(*) from consent_events')" "$EVENTS"
check "every contact's consent is unchanged" \
  "$(sql "select md5(string_agg(pid::text || marketing_consent, '|' order by pid)) from contacts")" "$CONSENT_HASH"
check "every event is unchanged" \
  "$(sql "select md5(string_agg(pid::text || action || source, '|' order by pid)) from consent_events")" "$EVENT_HASH"
check "every old event is a marketing event" "$(sql "select count(*) from consent_events where purpose <> 'marketing'")" 0
check "every campaign is sent under marketing" "$(sql "select count(*) from campaigns where purpose <> 'marketing'")" 0
check "every sequence is sent under marketing" "$(sql "select count(*) from nurture_sequences where purpose <> 'marketing'")" 0
check "no other purpose is held for anyone" "$(sql 'select count(*) from contact_consents')" 0
run db migrate | tail -1 | sed 's/^/  second run: /'

echo "== 4. the migration rolls back and forward again"
sql "insert into contact_consents (contact_pid, purpose, state) select pid, 'newsletter', 'granted' from contacts limit 3" >/dev/null
run db down 1 | tail -1
check "marketing consent survives the rollback" \
  "$(sql "select md5(string_agg(pid::text || marketing_consent, '|' order by pid)) from contacts")" "$CONSENT_HASH"
run db migrate | tail -1
check "forward again, events intact" "$(sql 'select count(*) from consent_events')" "$EVENTS"

[[ $fail = 0 ]] && echo "all checks passed" || { echo "SOME CHECKS FAILED"; exit 1; }
