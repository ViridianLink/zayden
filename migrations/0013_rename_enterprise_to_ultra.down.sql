ALTER TABLE entitlements
    DROP CONSTRAINT entitlements_tier_check;

UPDATE
    entitlements
SET
    tier = 'enterprise'
WHERE
    tier = 'ultra';

UPDATE
    entitlement_cache
SET
    tier = 'enterprise'
WHERE
    tier = 'ultra';

ALTER TABLE entitlements
    ADD CONSTRAINT entitlements_tier_check CHECK (tier IN ('free', 'pro', 'enterprise'));

