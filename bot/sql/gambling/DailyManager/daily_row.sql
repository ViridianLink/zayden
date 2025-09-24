SELECT
    g.id,
    g.coins,
    g.daily,
    m.prestige,
    COALESCE(
        jsonb_agg(
            DISTINCT jsonb_build_object(
                'user_id',
                gg.user_id,
                'goal_id',
                gg.goal_id,
                'day',
                gg.day,
                'progress',
                gg.progress,
                'target',
                gg.target
            )
        ) FILTER (
            WHERE
                gg.user_id IS NOT NULL
        ),
        '[]'::jsonb
    ) as "goals!: Json<Vec<GamblingGoalsRow>>"
FROM
    gambling g
    LEFT JOIN gambling_mine m on g.id = m.id
    LEFT JOIN gambling_goals gg ON g.id = gg.user_id
WHERE
    g.id = $1
GROUP BY
    g.id,
    m.prestige