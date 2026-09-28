ALTER TABLE hosted_servers
    ADD COLUMN unsuspend_due timestamptz;

CREATE INDEX hosted_servers_unsuspend_idx ON hosted_servers (unsuspend_due)
WHERE
    unsuspend_due IS NOT NULL;

