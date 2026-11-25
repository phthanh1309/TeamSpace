-- Opaque server-side sessions for TeamSpace.
--
-- A session token is generated with a cryptographically secure random source
-- and returned to the client exactly once. Only its SHA-256 hash is stored, so
-- a database leak does not expose usable bearer tokens.
CREATE TABLE sessions (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMPTZ NOT NULL
);

-- Authentication performs a point lookup on token_hash. The unique index both
-- enforces one row per hash and serves those lookups.
CREATE UNIQUE INDEX sessions_token_hash_key ON sessions (token_hash);

-- Foreign-key maintenance and cascading deletes look rows up by user_id.
CREATE INDEX sessions_user_id_idx ON sessions (user_id);
