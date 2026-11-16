-- Outbox: events written in the same transaction as the data they describe,
-- then handled by the outbox worker (handlers/outbox_handler.rs).
CREATE TABLE IF NOT EXISTS outbox (
    id           BIGINT NOT NULL AUTO_INCREMENT,
    event_type   VARCHAR(100) NOT NULL,
    payload      JSON NOT NULL,          -- MariaDB: text checked with JSON_VALID
    created_at   DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    published_at DATETIME(6) NULL,       -- set when handled successfully
    attempts     INT NOT NULL DEFAULT 0,
    last_error   TEXT NULL,
    failed_at    DATETIME(6) NULL,       -- set when we gave up (too many attempts)
    PRIMARY KEY (id),
    -- The worker only searches pending events (both dates NULL), in id order.
    KEY outbox_pending_idx (published_at, failed_at, id)
);

CREATE OR REPLACE PROCEDURE sp_insert_outbox_event(
    IN prm_event_type VARCHAR(100),
    IN prm_payload LONGTEXT
)
BEGIN
    INSERT INTO outbox (event_type, payload)
    VALUES (prm_event_type, prm_payload);
END;

-- Takes the next pending event after `prm_after_id`, counts one more attempt and
-- returns it (or nothing). FOR UPDATE SKIP LOCKED (MariaDB 10.6+): the row stays locked
-- until the caller's transaction ends, and a second worker skips it instead of waiting.
-- Call it inside a transaction, handle the event, mark it, then commit.
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

CREATE OR REPLACE PROCEDURE sp_mark_outbox_published(IN prm_id BIGINT)
BEGIN
    UPDATE outbox
    SET published_at = NOW(6), last_error = NULL
    WHERE id = prm_id;
END;

-- Records a failed attempt and returns one row: gave_up = 1 when this was the last
-- allowed attempt (the event is set aside: failed_at is filled, never claimed again).
CREATE OR REPLACE PROCEDURE sp_mark_outbox_failed(
    IN prm_id BIGINT,
    IN prm_error TEXT,
    IN prm_max_attempts INT
)
BEGIN
    UPDATE outbox
    SET last_error = prm_error,
        failed_at = IF(attempts >= prm_max_attempts, NOW(6), NULL)
    WHERE id = prm_id;

    SELECT CAST(failed_at IS NOT NULL AS SIGNED) AS gave_up
    FROM outbox
    WHERE id = prm_id;
END;
