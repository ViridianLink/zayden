DROP INDEX IF EXISTS hosted_servers_unsuspend_idx;

ALTER TABLE hosted_servers
    DROP COLUMN IF EXISTS unsuspend_due;

