-- No "DELIMITER" here: that's a feature of the mysql command-line client. sqlx sends
-- the whole file to the server, which understands BEGIN ... END by itself.
CREATE OR REPLACE PROCEDURE sp_Return_USERS()
BEGIN
    SELECT
        U.UID,
        U.NAME
    FROM t_Users U;
END;
