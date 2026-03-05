-- ============================================
-- Owner License
-- Full access for app owner: no expiry, no reset, no autopay
-- ============================================

-- Add is_owner to licenses (owner licenses are never modified by payment flow)
ALTER TABLE licenses ADD COLUMN IF NOT EXISTS is_owner BOOLEAN DEFAULT false;

-- Add is_owner to users (owner users skip token limit and monthly reset)
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_owner BOOLEAN DEFAULT false;

-- Modify trigger: do NOT reset tokens for owner users
CREATE OR REPLACE FUNCTION reset_monthly_tokens() RETURNS TRIGGER AS $$
BEGIN
    -- Owner: never reset
    IF NEW.is_owner = true THEN
        RETURN NEW;
    END IF;
    IF NEW.monthly_reset_at < NOW() THEN
        NEW.tokens_used_this_month := 0;
        NEW.monthly_reset_at := DATE_TRUNC('month', NOW() + INTERVAL '1 month');
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create owner user and license if not exists
-- License key: GHOST-OWNER-00000000
-- To use your existing license: UPDATE licenses SET license_key = 'YOUR-KEY' WHERE is_owner = true;
DO $$
DECLARE
    owner_user_id UUID;
BEGIN
    -- Create owner user if none exists
    IF NOT EXISTS (SELECT 1 FROM users WHERE is_owner = true) THEN
        owner_user_id := gen_random_uuid();
        INSERT INTO users (
            id, email, plan, monthly_token_limit, tokens_used_this_month,
            monthly_reset_at, is_owner, created_at, updated_at
        ) VALUES (
            owner_user_id,
            'owner@ghost.local',
            'power',
            999999999,
            0,
            '2099-12-31 23:59:59+00',
            true,
            NOW(),
            NOW()
        );
    ELSE
        SELECT id INTO owner_user_id FROM users WHERE is_owner = true LIMIT 1;
    END IF;

    -- Create owner license if none exists
    IF NOT EXISTS (SELECT 1 FROM licenses WHERE is_owner = true) THEN
        INSERT INTO licenses (
            id, license_key, user_id, status, tier, max_instances,
            is_trial, trial_ends_at, expires_at, is_owner, created_at, updated_at
        ) VALUES (
            gen_random_uuid(),
            'GHOST-OWNER-00000000',
            owner_user_id,
            'active',
            'power',
            99,
            false,
            NULL,
            NULL,
            true,
            NOW(),
            NOW()
        );
    END IF;
END $$;
