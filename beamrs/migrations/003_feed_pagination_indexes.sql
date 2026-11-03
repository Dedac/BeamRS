DROP INDEX IF EXISTS rays_frequency_created_idx;
DROP INDEX IF EXISTS rays_user_created_idx;

CREATE INDEX IF NOT EXISTS rays_created_id_idx
    ON rays (created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS rays_frequency_created_id_idx
    ON rays (frequency_id, created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS rays_user_created_id_idx
    ON rays (user_id, created_at DESC, id DESC);
