CREATE FUNCTION attach_config_notify (tbl regclass)
    RETURNS void
    AS $$
BEGIN
    EXECUTE format('CREATE OR REPLACE TRIGGER %I AFTER INSERT OR UPDATE OR DELETE ON %s FOR EACH ROW EXECUTE FUNCTION notify_config_changed ()', (
            SELECT
                relname || '_notify'
            FROM pg_class
            WHERE
                oid = tbl), tbl);
END;
$$
LANGUAGE plpgsql;

SELECT
    attach_config_notify ('family_settings');

