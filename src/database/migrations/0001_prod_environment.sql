-- Applied at startup by sqlx (see database::connection::run_migrations), on whatever
-- database the connection points to (TESTMS, DEV_ENVIRONMENT, test_db...): no
-- "CREATE DATABASE" / "USE" here, MariaDB creates the database from MARIADB_DATABASE.
-- Safe on a database that already has these objects (created by the old init scripts).
-- Never edit this file once it has been applied: add a new numbered file instead.
CREATE TABLE IF NOT EXISTS t_Users (
    UID int(11) NOT NULL AUTO_INCREMENT,
    NAME varchar(15) NOT NULL,
    PRIMARY KEY (UID)
);

-- Seed only an empty table, so an existing database doesn't get every name twice.
INSERT INTO t_Users (NAME)
SELECT seed.NAME
FROM (
    SELECT 'Alice' AS NAME UNION ALL
    SELECT 'Bob' UNION ALL
    SELECT 'Charlie' UNION ALL
    SELECT 'David' UNION ALL
    SELECT 'Eve' UNION ALL
    SELECT 'Frank' UNION ALL
    SELECT 'Grace' UNION ALL
    SELECT 'Heidi' UNION ALL
    SELECT 'Ivan' UNION ALL
    SELECT 'Judy' UNION ALL
    SELECT 'Karl' UNION ALL
    SELECT 'Leo' UNION ALL
    SELECT 'Mallory' UNION ALL
    SELECT 'Nina' UNION ALL
    SELECT 'Oscar' UNION ALL
    SELECT 'Peggy' UNION ALL
    SELECT 'Quentin'
) AS seed
WHERE NOT EXISTS (SELECT 1 FROM t_Users);
