CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username TEXT NOT NULL UNIQUE
        CHECK (char_length(username) BETWEEN 1 AND 40)
        CHECK (username ~ '^[A-Za-z0-9_-]+$'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS frequencies (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE CHECK (char_length(name) BETWEEN 1 AND 60),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS rays (
    id SERIAL PRIMARY KEY,
    frequency_id INTEGER NOT NULL REFERENCES frequencies(id) ON DELETE CASCADE,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    text TEXT NOT NULL CHECK (char_length(text) BETWEEN 1 AND 300),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS prisms (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ray_id INTEGER NOT NULL REFERENCES rays(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, ray_id)
);

CREATE INDEX IF NOT EXISTS rays_frequency_created_idx
    ON rays (frequency_id, created_at DESC);
CREATE INDEX IF NOT EXISTS rays_user_created_idx
    ON rays (user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS prisms_ray_idx ON prisms (ray_id);
