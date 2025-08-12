INSERT INTO
    gambling_inventory (user_id, item_id, quantity)
VALUES
    ($1, $2, $3) ON CONFLICT (user_id, item_id)
DO
UPDATE
SET
    quantity = EXCLUDED.quantity + $3