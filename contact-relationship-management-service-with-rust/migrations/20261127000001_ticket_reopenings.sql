-- CRM-T317: each resolution a reopen undid, so a reopened ticket keeps when it was resolved before
-- and how many times it came back. Append-only, as ticket_movements: never changed, and gone only
-- with its ticket. No free text, so erasure has nothing to scrub here.
CREATE TABLE ticket_reopenings (
    id bigserial PRIMARY KEY,
    ticket_pid uuid NOT NULL REFERENCES tickets (pid) ON DELETE CASCADE,
    resolved_at timestamptz NOT NULL,
    reopened_at timestamptz NOT NULL DEFAULT now(),
    actor_ref varchar
);

CREATE INDEX ticket_reopenings_by_ticket ON ticket_reopenings (ticket_pid, id);

CREATE FUNCTION ticket_reopenings_append_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' AND NOT EXISTS (SELECT 1 FROM tickets WHERE pid = OLD.ticket_pid) THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'ticket_reopenings is append-only (CRM-T317)';
END
$$;

CREATE TRIGGER ticket_reopenings_append_only BEFORE UPDATE OR DELETE ON ticket_reopenings
    FOR EACH ROW EXECUTE FUNCTION ticket_reopenings_append_only();
