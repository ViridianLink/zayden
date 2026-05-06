INSERT INTO
    gambling_stats (
        user_id,
        higher_or_lower_score,
        weekly_higher_or_lower_score
    )
VALUES
    ($1, $2, $2) ON CONFLICT (user_id)
DO
UPDATE
SET
    higher_or_lower_score = GREATEST(
        gambling_stats.higher_or_lower_score,
        EXCLUDED.higher_or_lower_score
    ),
    weekly_higher_or_lower_score = GREATEST(
        gambling_stats.weekly_higher_or_lower_score,
        EXCLUDED.weekly_higher_or_lower_score
    );