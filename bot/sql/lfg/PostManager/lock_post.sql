SELECT
    id
FROM
    lfg_posts
WHERE
    id = $1
FOR UPDATE
