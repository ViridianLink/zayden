ALTER TABLE entitlements
    DROP CONSTRAINT entitlements_tier_check;

UPDATE
    entitlements
SET
    tier = 'ultra'
WHERE
    tier = 'enterprise';

UPDATE
    entitlement_cache
SET
    tier = 'ultra'
WHERE
    tier = 'enterprise';

ALTER TABLE entitlements
    ADD CONSTRAINT entitlements_tier_check CHECK (tier IN ('free', 'pro', 'ultra'));

