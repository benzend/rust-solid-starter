CREATE TABLE todos (
    id         INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    title      TEXT        NOT NULL CHECK (char_length(title) BETWEEN 1 AND 200),
    done       BOOLEAN     NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
