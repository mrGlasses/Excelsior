DELIMITER //

CREATE OR REPLACE PROCEDURE TESTMS.sp_Insert_User(
    IN prmName varchar(15)
)
BEGIN
    START TRANSACTION;
    INSERT INTO t_Users (NAME)
    VALUES (prmName);
    COMMIT;

    #    SIGNAL SQLSTATE '45000'
    #		SET MESSAGE_TEXT = 'TEST';
end //

DELIMITER ;

#the "signal" statements above are commented out to avoid errors during execution.