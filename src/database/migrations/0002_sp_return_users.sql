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
