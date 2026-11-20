-- CRM-R105, CRM-T282: the name of the deployment this database belongs to, written at the first
-- start that has `CRM_DEPLOYMENT_ID` and checked at every start after, so a configuration mistake
-- cannot point one customer's service at another customer's database. One row at most, and it
-- cannot be changed or removed once written (a TRUNCATE by a database owner still can).
CREATE TABLE deployment (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    name text NOT NULL CHECK (name ~ '^[A-Za-z0-9._-]{1,64}$'),
    claimed_at timestamptz NOT NULL DEFAULT now()
);

CREATE FUNCTION deployment_is_fixed() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'the deployment name is fixed once written (CRM-R105)';
END
$$;

CREATE TRIGGER deployment_fixed BEFORE UPDATE OR DELETE ON deployment
    FOR EACH ROW EXECUTE FUNCTION deployment_is_fixed();
