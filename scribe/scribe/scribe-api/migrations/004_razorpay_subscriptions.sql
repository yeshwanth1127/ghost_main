-- ============================================
-- Razorpay Subscriptions & Customer Auth
-- Adds password for customers, Razorpay fields
-- ============================================

-- Add password_hash for customer email+password auth (nullable for existing users)
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_hash TEXT;

-- Add Razorpay fields for subscription tracking
ALTER TABLE users ADD COLUMN IF NOT EXISTS razorpay_subscription_id TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS razorpay_customer_id TEXT;
