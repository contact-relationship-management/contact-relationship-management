-- CRM-T317: who a ticket was handed to, from whom, when, by whom and why, one row a move. The
-- history is append-only: a row is never changed, except that erasure scrubs its free-text reason
-- to the tombstone, and it goes only with its ticket, which the retention sweep hard-deletes.
CREATE TABLE ticket_movements (
    id bigserial PRIMARY KEY,
    ticket_pid uuid NOT NULL REFERENCES tickets (pid) ON DELETE CASCADE,
    from_ref varchar,
    to_ref varchar,
    -- 'person': a person through the API; 'escalation': a sweep on its policy's wait limit.
    via varchar NOT NULL CHECK (via IN ('person', 'escalation')),
    reason text NOT NULL,
    actor_ref varchar,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ticket_movements_by_ticket ON ticket_movements (ticket_pid, id);

CREATE FUNCTION ticket_movements_append_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE'
       AND NEW.reason = '[erased]'
       AND (NEW.id, NEW.ticket_pid, NEW.from_ref, NEW.to_ref, NEW.via, NEW.actor_ref, NEW.created_at)
           IS NOT DISTINCT FROM
           (OLD.id, OLD.ticket_pid, OLD.from_ref, OLD.to_ref, OLD.via, OLD.actor_ref, OLD.created_at) THEN
        RETURN NEW;
    END IF;
    IF TG_OP = 'DELETE' AND NOT EXISTS (SELECT 1 FROM tickets WHERE pid = OLD.ticket_pid) THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'ticket_movements is append-only (CRM-T317)';
END
$$;

CREATE TRIGGER ticket_movements_append_only BEFORE UPDATE OR DELETE ON ticket_movements
    FOR EACH ROW EXECUTE FUNCTION ticket_movements_append_only();
