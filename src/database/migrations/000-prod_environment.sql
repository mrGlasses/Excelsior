CREATE DATABASE IF NOT EXISTS TESTMS
CHARACTER SET = 'utf8mb4'
COLLATE = 'utf8mb4_unicode_520_ci';

USE TESTMS;

CREATE TABLE `TESTMS`.`t_Users` (
    `UID` int(11) NOT NULL AUTO_INCREMENT,
    `NAME` varchar(15) NOT NULL,
    PRIMARY KEY (`UID`)
);

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



