CREATE TABLE users(
    id UUID PRIMARY KEY,
    user_name VARCHAR(40) NOT NULL,
    email_address VARCHAR(120) NOT NULL UNIQUE,
    hashed_password TEXT NOT NULL,
    is_active BOOLEAN NOT NULL,
    last_logged_in TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);
