#!/usr/bin/env bash
# Migration 54 (CRM users named by person:, CRM-D18) against a database that already holds data.
# Builds a database at migration 53 holding N deals, contacts and tasks owned by worker: references,
# quotas, a split and a clawback, a saved view and a report, a workflow, a team set by hand and a
# team set by the WPM import (with a real WPM worker pid), and history (an audit row, a run log).
# Migrates forward with the data in place and checks every user reference became person: with the
# same id, that nothing else changed (ids, counts, updated_at, history, the imported team), that the
# imported team's roll-up now meets the quotas, and that the migration rolls back and forward again.
# Usage: N=500 scripts/migrate-person-refs.sh   (needs psql and the debug binary; a Postgres
# reachable with PG_HOST, PG_PORT, PG_USER and PG_PASSWORD, defaulting to localhost:5432 loco/loco;
# PG_ADMIN_USER, defaulting to PG_USER, creates and drops the database)
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
SVC="$(pwd)/contact-relationship-management-service-with-rust"
BIN=$SVC/target/debug/contact-relationship-management
: "${PG_HOST:=localhost}" "${PG_PORT:=5432}" "${PG_USER:=loco}" "${PG_PASSWORD:=loco}"
: "${PG_ADMIN_USER:=$PG_USER}"
export PGPASSWORD="$PG_PASSWORD"
DB=crm_mig_person_refs
export DATABASE_URL="postgres://$PG_USER:$PG_PASSWORD@$PG_HOST:$PG_PORT/$DB"
N=${N:-500}
admin() { psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_ADMIN_USER" -d "${2:-postgres}" -q -c "$1"; }
sql() { psql -h "$PG_HOST" -p "$PG_PORT" -U "$PG_USER" -d "$DB" -Atc "$1"; }
run() { (cd "$SVC" && "$BIN" "$@" 2>&1); }
fail=0
ok() { echo "  ok    $1"; }
bad() { echo "  FAIL  $1"; fail=1; }
check() { [[ "$2" = "$3" ]] && ok "$1" || bad "$1 (got $2, want $3)"; }
trap 'admin "DROP DATABASE IF EXISTS $DB" >/dev/null 2>&1' EXIT
admin "DROP DATABASE IF EXISTS $DB" >/dev/null 2>&1
admin "CREATE DATABASE $DB OWNER $PG_USER" || exit 1
admin 'CREATE EXTENSION IF NOT EXISTS "uuid-ossp"; CREATE EXTENSION IF NOT EXISTS citext; CREATE EXTENSION IF NOT EXISTS unaccent;' "$DB" >/dev/null 2>&1

echo "== 1. migrate to the latest, then return to migration 53"
run db migrate | tail -1
run db down 1 | tail -1

U1=0c4f1e2a-0000-4000-8000-0000000000a1   # a seller: a sign-in subject, so a person id
U2=0c4f1e2a-0000-4000-8000-0000000000a2   # another seller
WPM=0c4f1e2a-0000-4000-8000-0000000000f1  # a WPM worker pid, which is not a person id
P1=$U1 P2=$U2                              # what WPM's person_ref says for the two sellers

echo "== 2. fill it: $N deals, contacts and tasks owned by worker: references, and the rest"
sql "insert into pipelines (pid, name, created_at, updated_at) values (uuid_generate_v4(), 'P', now(), now())" >/dev/null
PIPE=$(sql "select pid from pipelines limit 1")
sql "insert into pipeline_stages (pid, pipeline_pid, name, position, probability_percent, is_won, is_lost, created_at, updated_at)
     values (uuid_generate_v4(), '$PIPE', 'Won', 0, 100, true, false, now(), now())" >/dev/null
STAGE=$(sql "select pid from pipeline_stages limit 1")
sql "insert into deals (pid, owner_ref, pipeline_pid, stage_pid, name, amount_minor, currency, kanban_position, closed_at, won, created_at, updated_at)
     select uuid_generate_v4(), 'worker:' || (array['$U1','$U2'])[1 + i % 2], '$PIPE', '$STAGE', 'Deal ' || i, 1000, 'GBP', 0, now(), true,
            now() - interval '3 days', now() - interval '2 days'
     from generate_series(1, $N) i" >/dev/null
sql "insert into contacts (pid, person_ref, display_name, status, preferred_channel, marketing_consent, owner_ref, created_at, updated_at)
     select uuid_generate_v4(), 'person:' || uuid_generate_v4(), 'Person ' || i, 'active', 'email', 'never',
            case when i % 5 = 0 then null else 'worker:$U1' end, now() - interval '3 days', now() - interval '2 days'
     from generate_series(1, $N) i" >/dev/null
sql "insert into activities (pid, subject_kind, subject_pid, kind, summary, actor_ref, assignee_ref, occurred_at, created_at, updated_at)
     select uuid_generate_v4(), 'contact', pid, 'task', 'call worker:$U2', 'worker:$U1', 'worker:$U2', now(), now(), now() - interval '2 days'
     from contacts" >/dev/null
sql "insert into notifications (pid, recipient_ref, kind, subject_kind, subject_pid, source_pid, source_key)
     select uuid_generate_v4(), 'worker:$U2', 'task_assigned', 'activity', pid, pid, 'assigned' from activities limit 50" >/dev/null
# A hand-made pair the API could not make: the rewrite would collide with a person: row, so it is skipped.
sql "insert into notifications (pid, recipient_ref, kind, subject_kind, subject_pid, source_pid, source_key)
     values (uuid_generate_v4(), 'worker:$U1', 'collide', 'deal', '$U1', '$U1', 'k'),
            (uuid_generate_v4(), 'person:$U1', 'collide', 'deal', '$U1', '$U1', 'k')" >/dev/null
TODAY=$(date +%F)
sql "insert into quotas (pid, owner_ref, period_from, period_to, currency, amount_minor, set_by)
     values (uuid_generate_v4(), 'worker:$U1', '$TODAY'::date - 10, '$TODAY'::date + 10, 'GBP', 100000, 'worker:$U1'),
            (uuid_generate_v4(), 'worker:$U2', '$TODAY'::date - 10, '$TODAY'::date + 10, 'GBP', 100000, 'worker:$U1')" >/dev/null
DEAL=$(sql "select pid from deals order by id limit 1")
sql "insert into deal_splits (deal_pid, owner_ref, share_bps, set_by) values ('$DEAL', 'worker:$U1', 6000, 'worker:$U1'), ('$DEAL', 'worker:$U2', 4000, 'worker:$U1')" >/dev/null
sql "insert into deal_clawbacks (deal_pid, owner_ref, amount_minor, reason, set_by) values ('$DEAL', 'worker:$U1', 10, 'refund', 'worker:$U1')" >/dev/null
sql "insert into saved_views (pid, owner_ref, kind, name, filter) values (uuid_generate_v4(), 'worker:$U1', 'deal', 'Mine', '{}')" >/dev/null
sql "insert into workflows (pid, name, trigger, actions) values (uuid_generate_v4(), 'Route', 'lead_created',
     '[{\"type\":\"assign_owner\",\"pool\":[\"worker:$U1\",\"worker:$U2\"],\"strategy\":\"round_robin\",\"notify\":true},
       {\"type\":\"create_task\",\"summary\":\"worker:$U1\",\"assignee_ref\":\"worker:$U1\"},
       {\"type\":\"notify\",\"recipient_ref\":\"worker:$U2\"},
       {\"type\":\"set_field\",\"field\":\"owner_ref\",\"value\":\"worker:$U2\"},
       {\"type\":\"set_field\",\"field\":\"job_title\",\"value\":\"worker:$U2\"}]')" >/dev/null
HAND=$(uuidgen | tr A-Z a-z) IMPORTED=$(uuidgen | tr A-Z a-z)
sql "insert into teams (pid, name, members, set_by) values
       ('$HAND', 'North', '[\"worker:$U1\",\"worker:$U2\"]', 'worker:$U1'),
       ('$IMPORTED', 'Support', '[\"worker:$WPM\"]', 'worker:$U1')" >/dev/null
sql "insert into audit_logs (entity, entity_pid, action, actor, snapshot, created_at, updated_at) values
       ('team', '$HAND', 'set', '$U1', '{\"name\":\"North\"}', now(), now()),
       ('team', '$IMPORTED', 'set', '$U1', '{\"name\":\"Support\",\"via\":\"wpm\"}', now(), now()),
       ('deal', '$DEAL', 'updated', '$U1', '{\"owner_ref\":\"worker:$U1\"}', now(), now())" >/dev/null
AUDIT_HASH=$(sql "select md5(string_agg(coalesce(snapshot::text,''), '|' order by id)) from audit_logs")
DEALS=$(sql "select count(*) from deals"); ACTS=$(sql "select count(*) from activities")
STAMPS=$(sql "select md5(string_agg(pid::text || updated_at::text, '|' order by pid)) from (select pid, updated_at from deals union all select pid, updated_at from contacts union all select pid, updated_at from activities) s")
IDS=$(sql "select md5(string_agg(pid::text || coalesce(substr(owner_ref, 8), '-'), '|' order by pid)) from deals")
echo "  deals: $DEALS  contacts: $(sql 'select count(*) from contacts')  activities: $ACTS  notifications: $(sql 'select count(*) from notifications')"

echo "== 3. migrate up with the data in place"
run db migrate | tail -1
left() { sql "select
  (select count(*) from deals where owner_ref like 'worker:%') + (select count(*) from contacts where owner_ref like 'worker:%')
  + (select count(*) from activities where actor_ref like 'worker:%' or assignee_ref like 'worker:%')
  + (select count(*) from notifications where recipient_ref like 'worker:%' and kind <> 'collide')
  + (select count(*) from quotas where owner_ref like 'worker:%' or set_by like 'worker:%')
  + (select count(*) from deal_splits where owner_ref like 'worker:%' or set_by like 'worker:%')
  + (select count(*) from deal_clawbacks where owner_ref like 'worker:%' or set_by like 'worker:%')
  + (select count(*) from saved_views where owner_ref like 'worker:%')
  + (select count(*) from teams where set_by like 'worker:%')"; }
check "no user reference is left as worker:" "$(left)" 0
check "a rewrite that would collide is skipped, both rows kept" "$(sql "select string_agg(recipient_ref, ',' order by recipient_ref) from notifications where kind = 'collide'")" "person:$U1,worker:$U1"
check "no deal lost" "$(sql 'select count(*) from deals')" "$DEALS"
check "no activity lost" "$(sql 'select count(*) from activities')" "$ACTS"
check "every deal keeps its owner's id" "$(sql "select md5(string_agg(pid::text || coalesce(substr(owner_ref, 8), '-'), '|' order by pid)) from deals")" "$IDS"
check "a contact with no owner still has none" "$(sql "select count(*) from contacts where owner_ref is null")" "$((N / 5))"
check "updated_at did not move" "$(sql "select md5(string_agg(pid::text || updated_at::text, '|' order by pid)) from (select pid, updated_at from deals union all select pid, updated_at from contacts union all select pid, updated_at from activities) s")" "$STAMPS"
check "a task's summary is text, not rewritten" "$(sql "select count(*) from activities where summary = 'call worker:$U2'")" "$ACTS"
check "the hand-set team's members are person:" "$(sql "select members::text from teams where pid = '$HAND'")" "[\"person:$U1\", \"person:$U2\"]"
check "the imported team keeps its WPM worker pid" "$(sql "select members::text from teams where pid = '$IMPORTED'")" "[\"worker:$WPM\"]"
check "the workflow pool, assignee, recipient and owner are person:" \
  "$(sql "select (actions->0->'pool')::text || (actions->1->>'assignee_ref') || (actions->2->>'recipient_ref') || (actions->3->>'value') from workflows")" \
  "[\"person:$U1\", \"person:$U2\"]person:${U1}person:${U2}person:$U2"
check "the workflow's other values are untouched" "$(sql "select (actions->1->>'summary') || (actions->4->>'value') from workflows")" "worker:${U1}worker:$U2"
check "the audit log is untouched" "$(sql "select md5(string_agg(coalesce(snapshot::text,''), '|' order by id)) from audit_logs")" "$AUDIT_HASH"
check "the updated_at trigger is back on" "$(sql "select count(*) from pg_trigger where tgname = 'crm_touch_updated_at' and tgenabled = 'O' and tgrelid::regclass::text in ('accounts','activities','contacts','deals','leads','sla_policies','tickets')")" 7
# What an import after CRM-T161 stores: the sellers' person_refs. Its roll-up now meets their quotas.
sql "update teams set members = '[\"person:$P1\",\"person:$P2\"]' where pid = '$IMPORTED'" >/dev/null
check "quotas held by the imported team's members" "$(sql "select count(*) from quotas q join teams t on t.members ? q.owner_ref where t.pid = '$IMPORTED'")" 2
run db migrate | tail -1 | sed 's/^/  second run: /'

echo "== 4. the migration rolls back and forward again"
run db down 1 | tail -1
check "down puts worker: back on the deals" "$(sql "select count(*) from deals where owner_ref like 'worker:%'")" "$DEALS"
check "down leaves the WPM import's person_refs alone" "$(sql "select members::text from teams where pid = '$IMPORTED'")" "[\"person:$P1\", \"person:$P2\"]"
check "down puts the hand-set team back" "$(sql "select members::text from teams where pid = '$HAND'")" "[\"worker:$U1\", \"worker:$U2\"]"
run db migrate | tail -1
check "forward again, nothing left as worker:" "$(left)" 0
check "forward again, ids intact" "$(sql "select md5(string_agg(pid::text || coalesce(substr(owner_ref, 8), '-'), '|' order by pid)) from deals")" "$IDS"

[[ $fail = 0 ]] && echo "all checks passed" || { echo "SOME CHECKS FAILED"; exit 1; }
