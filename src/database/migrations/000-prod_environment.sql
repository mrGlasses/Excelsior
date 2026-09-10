-- Postgres creates the database itself from the POSTGRES_DB/DATABASE_NAME
-- env var before running init scripts, so this file only needs the schema.
CREATE TABLE IF NOT EXISTS t_users (
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
