UPDATE gambling
SET
    coins = coins - $2
WHERE
    user_id = $1;

