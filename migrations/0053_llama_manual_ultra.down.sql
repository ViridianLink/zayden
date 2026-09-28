DELETE FROM entitlements
WHERE provider = 'manual'
    AND external_id IN ('custom-bot:1133034263579734037', 'custom-bot:935189797528555610');

DELETE FROM entitlement_cache
WHERE scope_type = 'guild'
    AND scope_id IN (1133034263579734037, 935189797528555610);

