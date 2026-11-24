-- HFEX0004: a failed event waits before its next try, twice as long each time.
-- Before, every pass retried every pending event and every new event started a pass, so
-- a failing event could use all its attempts in a few seconds (a short outage of the
-- service it calls would lose it for good).
-- 0004 is never edited (applied files are checksummed): this file replaces the two
-- procedures that change.

ALTER TABLE outbox ADD COLUMN IF NOT EXISTS next_attempt_at DATETIME(6) NULL AFTER attempts;

-- Same as in 0004, plus: an event waiting for its next try (next_attempt_at in the
-- future) is skipped.
CREATE OR REPLACE PROCEDURE sp_claim_outbox_event(IN prm_after_id BIGINT)
BEGIN
    DECLARE v_id BIGINT DEFAULT NULL;
    -- No pending event: SELECT ... INTO finds no row; just leave v_id NULL.
    DECLARE CONTINUE HANDLER FOR NOT FOUND SET v_id = NULL;

    SELECT o.id INTO v_id
    FROM outbox o
    WHERE o.published_at IS NULL
      AND o.failed_at IS NULL
      AND o.id > prm_after_id
      AND (o.next_attempt_at IS NULL OR o.next_attempt_at <= NOW(6))
    ORDER BY o.id
    LIMIT 1
    FOR UPDATE SKIP LOCKED;

    IF v_id IS NOT NULL THEN
        UPDATE outbox SET attempts = attempts + 1 WHERE id = v_id;
        SELECT id AS event_id, event_type, CAST(payload AS CHAR) AS payload, attempts
        FROM outbox
        WHERE id = v_id;
    END IF;
END;

-- Records a failed attempt and returns one row: gave_up = 1 when this was the last
-- allowed attempt (failed_at is filled, never claimed again). Otherwise the next try
-- waits prm_retry_base_secs * 2^(attempts - 1) seconds, at most one hour:
-- with a 5s base, 5s, 10s, 20s... (10 attempts span about 43 minutes).
CREATE OR REPLACE PROCEDURE sp_mark_outbox_failed(
    IN prm_id BIGINT,
    IN prm_error TEXT,
    IN prm_max_attempts INT,
    IN prm_retry_base_secs INT
)
BEGIN
    UPDATE outbox
    SET last_error = prm_error,
        failed_at = IF(attempts >= prm_max_attempts, NOW(6), NULL),
        next_attempt_at = IF(
            attempts >= prm_max_attempts,
            NULL,
            NOW(6) + INTERVAL CAST(LEAST(prm_retry_base_secs * POW(2, attempts - 1), 3600) AS UNSIGNED) SECOND
        )
    WHERE id = prm_id;

    SELECT CAST(failed_at IS NOT NULL AS SIGNED) AS gave_up
    FROM outbox
    WHERE id = prm_id;
END;
