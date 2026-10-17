DROP INDEX deadlock_patches_unbriefed;

DROP TABLE deadlock_briefings_sent;

ALTER TABLE deadlock_user_settings
    DROP COLUMN patch_briefing;

