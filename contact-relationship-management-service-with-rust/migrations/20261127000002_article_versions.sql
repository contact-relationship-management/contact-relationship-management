-- CRM-T313: the text a published edit replaced, so earlier versions of a knowledge-base article stay
-- readable. One row per replaced version, written in the edit's transaction. Append-only: never
-- changed, and gone only with its article (the retention sweep hard-deletes a soft-deleted article,
-- and the rows go with it). How long old text is kept apart from that is the owner's to decide.
CREATE TABLE article_versions (
    id bigserial PRIMARY KEY,
    article_pid uuid NOT NULL REFERENCES articles (pid) ON DELETE CASCADE,
    version integer NOT NULL,
    title varchar NOT NULL,
    body text NOT NULL,
    keywords varchar,
    replaced_at timestamptz NOT NULL DEFAULT now(),
    replaced_by varchar,
    UNIQUE (article_pid, version)
);

CREATE FUNCTION article_versions_append_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' AND NOT EXISTS (SELECT 1 FROM articles WHERE pid = OLD.article_pid) THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'article_versions is append-only (CRM-T313)';
END
$$;

CREATE TRIGGER article_versions_append_only BEFORE UPDATE OR DELETE ON article_versions
    FOR EACH ROW EXECUTE FUNCTION article_versions_append_only();
