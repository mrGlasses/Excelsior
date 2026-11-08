-- Postgres creates the database itself from the POSTGRES_DB/DATABASE_NAME
-- env var, so this file only needs the schema.
-- Applied at startup by sqlx (see database::connection::run_migrations).
-- Never edit this file once it has been applied: add a new numbered file instead.
CREATE TABLE IF NOT EXISTS t_users (
    uid SERIAL PRIMARY KEY,
    name VARCHAR(15) NOT NULL
);

-- Seed only an empty table, so a database that was set up by hand
-- before migrations existed doesn't get every name twice.
INSERT INTO t_users (name)
SELECT seed.name
FROM (VALUES
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
    ('Quentin')
) AS seed(name)
WHERE NOT EXISTS (SELECT 1 FROM t_users);
