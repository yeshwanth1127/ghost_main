-- ============================================
-- Usage Tracking Migration
-- Adds comprehensive usage tracking, token limits, and cost management
-- ============================================

-- ============================================
-- 1. EXTEND USERS TABLE
-- ============================================
ALTER TABLE users ADD COLUMN IF NOT EXISTS plan TEXT DEFAULT 'free' CHECK (plan IN ('free', 'starter', 'pro', 'power'));
ALTER TABLE users ADD COLUMN IF NOT EXISTS monthly_token_limit BIGINT DEFAULT 5000;
ALTER TABLE users ADD COLUMN IF NOT EXISTS tokens_used_this_month BIGINT DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS monthly_reset_at TIMESTAMPTZ DEFAULT DATE_TRUNC('month', NOW() + INTERVAL '1 month');
ALTER TABLE users ADD COLUMN IF NOT EXISTS stripe_customer_id TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS stripe_subscription_id TEXT;

-- ============================================
-- 2. MESSAGES TABLE (track every AI request)
-- ============================================
CREATE TABLE IF NOT EXISTS messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    license_key TEXT,
    
    -- Model info
    model TEXT NOT NULL,
    provider TEXT NOT NULL,
    
    -- Token usage
    prompt_tokens INT NOT NULL,
    completion_tokens INT NOT NULL,
    total_tokens INT NOT NULL,
    
    -- Cost
    cost_usd NUMERIC(10, 6) NOT NULL,
    cost_inr NUMERIC(10, 2),
    
    -- Request metadata
    conversation_id TEXT,
    request_duration_ms INT,
    status TEXT DEFAULT 'success' CHECK (status IN ('success', 'error', 'rate_limited')),
    error_message TEXT,
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================
-- 3. MONTHLY USAGE (aggregated stats)
-- ============================================
CREATE TABLE IF NOT EXISTS monthly_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    month TEXT NOT NULL, -- Format: 'YYYY-MM'
    
    total_tokens BIGINT DEFAULT 0,
    total_cost_usd NUMERIC(10, 2) DEFAULT 0,
    total_cost_inr NUMERIC(12, 2) DEFAULT 0,
    total_requests INT DEFAULT 0,
    
    -- Model breakdown (JSON)
    model_usage JSONB DEFAULT '{}',
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(user_id, month)
);

-- ============================================
-- 4. MODEL PRICING (cost per 1M tokens)
-- ============================================
CREATE TABLE IF NOT EXISTS model_pricing (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    model TEXT UNIQUE NOT NULL,
    provider TEXT NOT NULL,
    
    input_cost_per_1m NUMERIC(8, 4) NOT NULL,  -- $ per 1M input tokens
    output_cost_per_1m NUMERIC(8, 4) NOT NULL, -- $ per 1M output tokens
    
    active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================
-- 5. INDEXES
-- ============================================
CREATE INDEX IF NOT EXISTS idx_messages_user_id ON messages(user_id);
CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_user_created ON messages(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_license_key ON messages(license_key);
CREATE INDEX IF NOT EXISTS idx_monthly_usage_user_month ON monthly_usage(user_id, month);
CREATE INDEX IF NOT EXISTS idx_model_pricing_active ON model_pricing(active);
CREATE INDEX IF NOT EXISTS idx_users_plan ON users(plan);

-- ============================================
-- 6. SEED DEFAULT PRICING
-- ============================================
INSERT INTO model_pricing (model, provider, input_cost_per_1m, output_cost_per_1m) VALUES
    ('gpt-4o-mini', 'openai', 0.15, 0.60),
    ('gpt-4o', 'openai', 5.00, 15.00),
    ('gpt-4', 'openai', 30.00, 60.00),
    ('gpt-3.5-turbo', 'openai', 0.50, 1.50),
    ('claude-3-5-sonnet', 'anthropic', 3.00, 15.00),
    ('claude-3-haiku', 'anthropic', 0.80, 4.00),
    ('claude-3-opus', 'anthropic', 15.00, 75.00),
    ('claude-3-sonnet', 'anthropic', 3.00, 15.00),
    ('gemini-2.0-flash', 'google', 0.00, 0.00),
    ('gemini-1.5-pro', 'google', 1.25, 5.00)
ON CONFLICT (model) DO NOTHING;

-- ============================================
-- 7. TRIGGER: Auto-reset monthly tokens
-- ============================================
CREATE OR REPLACE FUNCTION reset_monthly_tokens() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.monthly_reset_at < NOW() THEN
        NEW.tokens_used_this_month := 0;
        NEW.monthly_reset_at := DATE_TRUNC('month', NOW() + INTERVAL '1 month');
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_reset_monthly_tokens ON users;
CREATE TRIGGER trigger_reset_monthly_tokens
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION reset_monthly_tokens();

-- ============================================
-- 8. UPDATE EXISTING USERS WITH DEFAULT LIMITS
-- ============================================
UPDATE users
SET 
    plan = COALESCE(plan, 'free'),
    monthly_token_limit = COALESCE(monthly_token_limit, 5000),
    tokens_used_this_month = COALESCE(tokens_used_this_month, 0),
    monthly_reset_at = COALESCE(monthly_reset_at, DATE_TRUNC('month', NOW() + INTERVAL '1 month'));

-- Adjust limits based on plan
UPDATE users
SET monthly_token_limit = 500000
WHERE plan = 'starter' AND monthly_token_limit != 500000;

UPDATE users
SET monthly_token_limit = 1000000
WHERE plan = 'pro' AND monthly_token_limit != 1000000;

UPDATE users
SET monthly_token_limit = 2000000
WHERE plan = 'power' AND monthly_token_limit != 2000000;

-- ============================================
-- 9. MIGRATION COMPLETE
-- ============================================
-- Summary:
-- ✅ Extended users table with plan and token limits
-- ✅ Created messages table for detailed tracking
-- ✅ Created monthly_usage for fast queries
-- ✅ Created model_pricing table
-- ✅ Added all necessary indexes
-- ✅ Seeded default pricing
-- ✅ Created auto-reset trigger
-- ✅ Updated existing users
