DELETE FROM lfg_fireteam
WHERE
    post = $1
    AND user_id = $2;