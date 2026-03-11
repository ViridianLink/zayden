DELETE FROM lfg_fireteam
WHERE
    post_id = $1
    AND user_id = $2;

