CREATE TABLE refresh_tokens (
    session_id TEXT PRIMARY KEY,
    refresh_token TEXT NOT NULL,
    expired_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expired_at ON refresh_tokens
USING btree (expired_at);


