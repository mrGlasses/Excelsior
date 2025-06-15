CREATE OR REPLACE DATABASE TESTMS
CHARACTER SET = 'utf8mb4'
COLLATE = 'utf8mb4_unicode_520_ci';

CREATE TABLE `TESTMS`.`t_Users` (
    `UID` int(11) NOT NULL AUTO_INCREMENT,
    `NAME` varchar(15) NOT NULL,
    PRIMARY KEY (`UID`)
) ENGINE=InnoDB AUTO_INCREMENT=18 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_520_ci;

INSERT INTO `TESTMS`.`t_Users` (`NAME`) VALUES
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

CREATE OR REPLACE PROCEDURE TESTMS.sp_Return_USERS()
BEGIN
SELECT
    U.UID,
    U.NAME
FROM t_Users U;

#     SIGNAL SQLSTATE '45000'
# 		SET MESSAGE_TEXT = 'TEST';
END;

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
end;

#the "signal" statements above are commented out to avoid errors during execution.



