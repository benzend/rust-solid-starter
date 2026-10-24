CREATE TABLE users (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- The app trims and lowercases emails before storing them.
    email         TEXT        NOT NULL UNIQUE CHECK (email = lower(email)),
    password_hash TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Session and reset tokens live in the browser or an email. Only their SHA-256
-- hashes are stored, so a leaked database or backup can't be used to log in.
CREATE TABLE sessions (
    token_hash BYTEA       PRIMARY KEY,
    user_id    BIGINT      NOT NULL REFERENCES users ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON sessions (user_id);
CREATE INDEX ON sessions (expires_at);

CREATE TABLE password_resets (
    token_hash BYTEA       PRIMARY KEY,
    user_id    BIGINT      NOT NULL REFERENCES users ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON password_resets (user_id);

-- Todos now belong to a user. Existing rows have no owner, so they're removed.
DELETE FROM todos;
ALTER TABLE todos ADD COLUMN user_id BIGINT NOT NULL REFERENCES users ON DELETE CASCADE;
CREATE INDEX ON todos (user_id);
