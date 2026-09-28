INSERT INTO entitlements (provider, external_id, scope_type, scope_id, tier)
VALUES
    ('manual', 'custom-bot:1133034263579734037', 'guild', 1133034263579734037, 'ultra'),
    ('manual', 'custom-bot:935189797528555610', 'guild', 935189797528555610, 'ultra')
ON CONFLICT (provider, external_id)
    DO NOTHING;

INSERT INTO entitlement_cache (scope_type, scope_id, tier)
VALUES
    ('guild', 1133034263579734037, 'ultra'),
    ('guild', 935189797528555610, 'ultra')
ON CONFLICT (scope_type, scope_id, scope_secondary_id)
    DO UPDATE SET
        tier = EXCLUDED.tier,
        refreshed_at = now();

