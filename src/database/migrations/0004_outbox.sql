-- Outbox: events written in the same transaction as the data they describe,
-- then handled by the outbox worker (handlers/outbox_handler.rs).
CREATE TABLE IF NOT EXISTS outbox (
    id           BIGSERIAL PRIMARY KEY,
    event_type   VARCHAR(100) NOT NULL,
    payload      JSONB NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ,          -- set when handled successfully
    attempts     INT NOT NULL DEFAULT 0,
    last_error   TEXT,
    failed_at    TIMESTAMPTZ           -- set when we gave up (too many attempts)
);

-- Only pending events are ever searched: keep that search small.
CREATE INDEX IF NOT EXISTS outbox_pending_idx
    ON outbox (id)
    WHERE published_at IS NULL AND failed_at IS NULL;

-- Wake the worker. Postgres delivers a NOTIFY only when the transaction commits,
-- so a rolled-back event never wakes anyone. Works for inserts from anywhere (psql too).
CREATE OR REPLACE FUNCTION outbox_notify()
RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify('outbox', NEW.id::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS outbox_notify_after_insert ON outbox;
CREATE TRIGGER outbox_notify_after_insert
    AFTER INSERT ON outbox
    FOR EACH ROW EXECUTE FUNCTION outbox_notify();

CREATE OR REPLACE PROCEDURE sp_insert_outbox_event(IN prm_event_type VARCHAR(100), IN prm_payload JSONB)
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO outbox (event_type, payload)
    VALUES (prm_event_type, prm_payload);
END;
$$;

-- Takes the next pending event after `prm_after_id` and counts one more attempt.
-- FOR UPDATE SKIP LOCKED: the row stays locked until the caller's transaction ends,
-- and a second worker skips it instead of waiting. Two workers never get the same event.
-- Call it inside a transaction, handle the event, mark it, then commit.
CREATE OR REPLACE FUNCTION sp_claim_outbox_event(prm_after_id BIGINT)
RETURNS TABLE (event_id BIGINT, event_type VARCHAR, payload JSONB, attempts INT) AS $$
#variable_conflict use_column
BEGIN
    RETURN QUERY
    UPDATE outbox o
    SET attempts = o.attempts + 1
    WHERE o.id = (
        SELECT p.id
        FROM outbox p
        WHERE p.published_at IS NULL
          AND p.failed_at IS NULL
          AND p.id > prm_after_id
        ORDER BY p.id
        LIMIT 1
        FOR UPDATE SKIP LOCKED
    )
    RETURNING o.id, o.event_type, o.payload, o.attempts;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE PROCEDURE sp_mark_outbox_published(IN prm_id BIGINT)
LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE outbox
    SET published_at = now(), last_error = NULL
    WHERE id = prm_id;
END;
$$;

-- Records a failed attempt. Returns true when this was the last allowed attempt
-- (the event is set aside: failed_at is filled and it's never claimed again).
CREATE OR REPLACE FUNCTION sp_mark_outbox_failed(prm_id BIGINT, prm_error TEXT, prm_max_attempts INT)
RETURNS BOOLEAN AS $$
DECLARE
    gave_up BOOLEAN;
BEGIN
    UPDATE outbox
    SET last_error = prm_error,
        failed_at = CASE WHEN attempts >= prm_max_attempts THEN now() ELSE NULL END
    WHERE id = prm_id
    RETURNING failed_at IS NOT NULL INTO gave_up;
    RETURN COALESCE(gave_up, FALSE);
END;
$$ LANGUAGE plpgsql;
