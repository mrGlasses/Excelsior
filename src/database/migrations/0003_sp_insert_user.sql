CREATE OR REPLACE PROCEDURE sp_insert_user(IN prm_name varchar(15))
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO t_users (name)
    VALUES (prm_name);
END;
$$;
