-- Postgres has no single-script equivalent of MySQL's "CREATE DATABASE; USE";
-- the target database (DEV_ENVIRONMENT locally, test_db in CI) must already
-- exist and be the database this script is run against.
DROP TABLE IF EXISTS t_users CASCADE;

CREATE TABLE t_users (
    uid SERIAL PRIMARY KEY,
    name VARCHAR(15) NOT NULL
);

INSERT INTO t_users (name) VALUES
('Alice'),
('Bob'),
('Charlie'),
('David'),
('Eve'),
('Frank'),
('Grace'),
('Heidi'),
('Ivan'),
('Judy'),
('Karl'),
('Leo'),
('Mallory'),
('Nina'),
('Oscar'),
('Peggy'),
('Quentin');

CREATE OR REPLACE FUNCTION sp_return_users()
RETURNS TABLE (uid integer, name varchar) AS $$
BEGIN
    RETURN QUERY
    SELECT
        u.uid,
        u.name
    FROM t_users u;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE PROCEDURE sp_insert_user(IN prm_name varchar(15))
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO t_users (name)
    VALUES (prm_name);
END;
$$;
