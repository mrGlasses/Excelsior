-- HFEX0004: a failed event waits before its next try, twice as long each time.
-- Before, every pass retried every pending event and every new event started a pass, so
-- a failing event could use all its attempts in a few seconds (a short outage of the
-- service it calls would lose it for good).
-- 0004 is never edited (applied files are checksummed): this file replaces what changes.

ALTER TABLE outbox ADD COLUMN IF NOT EXISTS next_attempt_at TIMESTAMPTZ;

-- Same as in 0004, plus: an event waiting for its next try (next_attempt_at in the
-- future) is skipped.
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
          AND (p.next_attempt_at IS NULL OR p.next_attempt_at <= now())
        ORDER BY p.id
        LIMIT 1
        FOR UPDATE SKIP LOCKED
    )
    RETURNING o.id, o.event_type, o.payload, o.attempts;
END;
$$ LANGUAGE plpgsql;

-- In Postgres a new parameter makes a NEW function next to the old one (overloading),
-- so the 3-parameter version from 0004 goes away explicitly.
DROP FUNCTION IF EXISTS sp_mark_outbox_failed(BIGINT, TEXT, INT);

-- Records a failed attempt. Returns true when this was the last allowed attempt (the
-- event is set aside: failed_at is filled and it's never claimed again). Otherwise the
-- next try waits prm_retry_base_secs * 2^(attempts - 1) seconds, at most one hour:
-- with a 5s base, 5s, 10s, 20s... (10 attempts span about 43 minutes).
CREATE OR REPLACE FUNCTION sp_mark_outbox_failed(
    prm_id BIGINT,
    prm_error TEXT,
    prm_max_attempts INT,
    prm_retry_base_secs INT
)
RETURNS BOOLEAN AS $$
DECLARE
    gave_up BOOLEAN;
BEGIN
    UPDATE outbox
    SET last_error = prm_error,
        failed_at = CASE WHEN attempts >= prm_max_attempts THEN now() ELSE NULL END,
        next_attempt_at = CASE
            WHEN attempts >= prm_max_attempts THEN NULL
            ELSE now() + make_interval(secs => LEAST(prm_retry_base_secs * power(2, attempts - 1), 3600))
        END
    WHERE id = prm_id
    RETURNING failed_at IS NOT NULL INTO gave_up;
    RETURN COALESCE(gave_up, FALSE);
END;
$$ LANGUAGE plpgsql;
