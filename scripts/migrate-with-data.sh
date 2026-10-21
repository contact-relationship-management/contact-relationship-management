#!/usr/bin/env bash
# Migration 17 (contact merges) against a database that already holds data (CRM-T64).
# Seeds N contacts (default 300) with activities, tickets, deals, consent, group seats and
# nurture enrolments through the API, rolls the schema back three migrations (what v0.1.1
# had: three migrations back), migrates forward with the data in place, then merges 150 pairs and checks.
# Usage: N=300 scripts/migrate-with-data.sh   (needs podman, curl and the debug binary)
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
SVC="$(pwd)/contact-relationship-management-service-with-rust"
BIN=$SVC/target/debug/contact-relationship-management
PG=crm-mig-pg RD=crm-mig-redis PGP=15440 RDP=16380 PORT=5181
export DATABASE_URL="postgres://loco:loco@localhost:$PGP/legacy" CRM_REDIS_URL="redis://localhost:$RDP"
API=http://localhost:$PORT
sql() { podman exec "$PG" psql -U loco -d legacy -Atc "$1"; }
run() { (cd "$SVC" && "$BIN" "$@" 2>&1); }
cleanup() { [[ -n "${SP:-}" ]] && kill "$SP" 2>/dev/null; podman rm -f $PG $RD >/dev/null 2>&1; }
trap cleanup EXIT
podman rm -f $PG $RD >/dev/null 2>&1
podman run -d --name $PG -e POSTGRES_USER=loco -e POSTGRES_PASSWORD=loco -e POSTGRES_DB=legacy -p $PGP:5432 postgres:18-alpine >/dev/null
podman run -d --name $RD -p $RDP:6379 redis:8-alpine >/dev/null
for _ in $(seq 40); do podman exec $PG pg_isready -U loco -d legacy >/dev/null 2>&1 && break; sleep 1; done

echo "== 1. build a populated database"
run db migrate | tail -1
(cd "$SVC" && exec "$BIN" start --server-and-worker --port $PORT >/tmp/mig-svc.log 2>&1) & SP=$!
for _ in $(seq 60); do curl -sf $API/_ping >/dev/null && break; sleep 1; done
post() { curl -s -X POST -H 'content-type: application/json' -d "$2" "$API$1"; }
pidof() { sed -n 's/.*"pid":"\([^"]*\)".*/\1/p' | head -1; }
PIPE=$(post /api/pipelines '{"name":"P","stages":[{"name":"Q","probability_percent":10},{"name":"Won","probability_percent":100,"is_won":true}]}' | pidof)
SEQ=$(post /api/nurture-sequences '{"name":"Welcome","steps":[{"delay_hours":0,"template_ref":"w1","body":"Hi"}]}' | pidof)
GRP=$(post /api/groups '{"name":"Standards","purpose":"x"}' | pidof)
N=${N:-300}
: >/tmp/mig-pids.txt
for i in $(seq $N); do
  P=$(post /api/contacts "{\"person_ref\":\"person:$(uuidgen | tr A-Z a-z)\",\"display_name\":\"Person $i\",\"job_title\":null}" | pidof)
  echo "$P" >>/tmp/mig-pids.txt
  case $((i % 3)) in 0) post /api/contacts/$P/consent '{"action":"granted","source":"seed"}' >/dev/null;; 1) post /api/contacts/$P/consent '{"action":"withdrawn","source":"seed"}' >/dev/null;; esac
  post /api/activities "{\"subject_kind\":\"contact\",\"subject_pid\":\"$P\",\"kind\":\"note\",\"summary\":\"note $i\"}" >/dev/null
  post /api/activities "{\"subject_kind\":\"contact\",\"subject_pid\":\"$P\",\"kind\":\"call\",\"summary\":\"call $i\"}" >/dev/null
  [[ $((i % 2)) = 0 ]] && post /api/tickets "{\"title\":\"T$i\",\"priority\":\"normal\",\"contact_pid\":\"$P\"}" >/dev/null
  [[ $((i % 4)) = 0 ]] && post /api/deals "{\"name\":\"D$i\",\"pipeline_pid\":\"$PIPE\",\"amount_minor\":1000,\"currency\":\"GBP\",\"primary_contact_pid\":\"$P\"}" >/dev/null
  [[ $((i % 5)) = 0 ]] && post /api/groups/$GRP/members "{\"contact_pid\":\"$P\"}" >/dev/null
  if [[ $((i % 3)) = 0 ]]; then post /api/nurture-sequences/$SEQ/enrollments "{\"contact_pid\":\"$P\"}" >/dev/null; fi
  post /api/leads "{\"display_name\":\"Lead $i\",\"source\":\"web\"}" >/dev/null
done
kill $SP; wait $SP 2>/dev/null; SP=""
echo "contacts: $(sql 'select count(*) from contacts')  activities: $(sql 'select count(*) from activities')  tickets: $(sql 'select count(*) from tickets')  deals: $(sql 'select count(*) from deals')  consent_events: $(sql 'select count(*) from consent_events')"

echo "== 2. return the schema to what v0.1.1 had (migration 16)"
run db down 3 | tail -3
check_16() { [[ "$(sql "select count(*) from information_schema.columns where table_name='contacts' and column_name='merged_into_pid'")" = 0 && "$(sql "select to_regclass('contact_merges') is null")" = t ]]; }
check_16 && echo "  ok    column and ledger are gone" || echo "  FAIL  still at 17"
TABLES=$(sql "select string_agg(tablename,' ') from pg_tables where schemaname='public' and tablename not in ('seaql_migrations')")
snap() { for t in $TABLES; do echo "$t $(sql "select count(*), md5(string_agg((to_jsonb(t) - 'merged_into_pid')::text, '|' order by (to_jsonb(t) - 'merged_into_pid')::text)) from $t t")"; done; }
snap >/tmp/mig-before.txt
echo "  rows snapshotted in $(wc -l </tmp/mig-before.txt | tr -d ' ') tables"

echo "== 3. migrate up with the data in place"
run db migrate | tail -2
sql "select count(*) from contacts where merged_into_pid is not null" | sed 's/^/  contacts already marked merged: /'
snap >/tmp/mig-after.txt
# contacts gain a column, so compare counts everywhere and full hashes on tables the migration does not touch
if diff <(awk '{print $1,$2}' /tmp/mig-before.txt) <(grep -v -e '^contact_merges ' -e '^external_links ' /tmp/mig-after.txt | awk '{print $1,$2}') >/dev/null; then echo "  ok    every table has the same row count"; else echo "  FAIL  row counts differ"; diff <(awk '{print $1,$2}' /tmp/mig-before.txt) <(awk '{print $1,$2}' /tmp/mig-after.txt); fi
BAD=$(diff /tmp/mig-before.txt <(grep -v -e '^contact_merges ' -e '^external_links ' /tmp/mig-after.txt) | wc -l | tr -d ' ')
[[ "$BAD" = 0 ]] && echo "  ok    every table's rows are identical apart from the new column" || echo "  FAIL  $BAD table hashes changed"
run db migrate | tail -1 | sed 's/^/  second run: /'

echo "== 4. merge on the migrated database"
(cd "$SVC" && exec "$BIN" start --server-and-worker --port $PORT >/tmp/mig-svc.log 2>&1) & SP=$!
for _ in $(seq 60); do curl -sf $API/_ping >/dev/null && break; sleep 1; done
PIDS=(); while read -r l; do PIDS+=("$l"); done </tmp/mig-pids.txt
merged=0; failed=0
for k in $(seq 0 2 $((N - 2))); do
  A=${PIDS[$k]}; B=${PIDS[$((k + 1))]}
  S=$(curl -s -o /tmp/mig-r.json -w '%{http_code}' -X POST -H 'content-type: application/json' -d "{\"into_pid\":\"$B\",\"reason\":\"migration test\",\"dry_run\":false}" $API/api/contacts/$A/merge)
  [[ $S = 200 ]] && merged=$((merged + 1)) || { failed=$((failed + 1)); echo "  merge $A -> $S $(cat /tmp/mig-r.json | cut -c1-120)"; }
done
echo "  merged: $merged  failed: $failed"
inv() { local what="$1" q="$2" want="$3"; local got; got=$(sql "$q"); [[ "$got" = "$want" ]] && echo "  ok    $what" || echo "  FAIL  $what (got $got, want $want)"; }
inv "ledger has one row per merge" "select count(*) from contact_merges" "$merged"
inv "no activity points at a merged-away contact" "select count(*) from activities a join contacts c on c.pid=a.subject_pid where a.subject_kind='contact' and c.merged_into_pid is not null" 0
inv "no ticket points at a merged-away contact" "select count(*) from tickets t join contacts c on c.pid=t.contact_pid where c.merged_into_pid is not null" 0
inv "no activities were lost" "select count(*) from activities" "$(awk '$1=="activities"{print $2}' /tmp/mig-before.txt | cut -d'|' -f1)"
inv "every merged contact is soft-deleted" "select count(*) from contacts where merged_into_pid is not null and deleted_at is null" 0
curl -s $API/api/audits/verify | grep -q '"ok":true' && echo "  ok    audit chain verifies" || echo "  FAIL  audit chain"
