-- Same behavior as before. Note: this procedure commits on its own (START TRANSACTION /
-- COMMIT inside), so it can't be part of a bigger transaction (a COMMIT inside would
-- commit the caller's transaction too). The outbox doesn't use it.
CREATE OR REPLACE PROCEDURE sp_Insert_User(
    IN prmName varchar(15)
)
BEGIN
    START TRANSACTION;
    INSERT INTO t_Users (NAME)
    VALUES (prmName);
    COMMIT;
END;
