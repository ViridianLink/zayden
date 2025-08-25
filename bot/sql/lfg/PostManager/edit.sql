UPDATE lfg_posts
SET
    owner = $2,
    activity = $3,
    start_time = $4,
    description = $5,
    fireteam_size = $6
WHERE
    id = $1;