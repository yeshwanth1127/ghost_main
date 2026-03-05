-- ============================================
-- Admin Users Table
-- For Ghost Admin Dashboard authentication
-- ============================================

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS admin_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Hardcoded admin: username=admin, password=ghostadmin123
INSERT INTO admin_users (username, password_hash)
VALUES ('admin', crypt('ghostadmin123', gen_salt('bf', 12)))
ON CONFLICT (username) DO NOTHING;
