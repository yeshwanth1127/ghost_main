# Usage Tracking - Quick Start Guide

## 📋 Prerequisites

- PostgreSQL database running
- Rust toolchain installed
- Node.js installed
- Environment variables configured

## 🚀 Deployment Steps

### 1. Database Setup

```bash
cd d:\ysw\ghost\ghost_main\scribe\scribe\scribe-api

# Run the migration
psql $DATABASE_URL -f migrations/002_usage_tracking.sql

# Verify tables were created
psql $DATABASE_URL -c "SELECT table_name FROM information_schema.tables WHERE table_schema='public' AND table_name IN ('messages', 'monthly_usage', 'model_pricing');"
```

Expected output:
```
    table_name    
------------------
 messages
 monthly_usage
 model_pricing
```

### 2. Backend Build

```bash
cd d:\ysw\ghost\ghost_main\scribe\scribe\scribe-api

# Check for compilation errors
cargo check

# Build release binary
cargo build --release

# Run backend (development)
cargo run
```

Backend should start on `http://localhost:3000`

### 3. Frontend Build

```bash
cd d:\ysw\ghost\ghost_main\scribe\scribe

# Install dependencies (if needed)
npm install

# Build frontend
npm run build

# Run development server
npm run dev
```

## 🧪 Testing

### Test 1: Database Migration

```bash
# Connect to database
psql $DATABASE_URL

# Check users table has new columns
\d+ users

# Check model_pricing has seed data
SELECT model, provider, input_cost_per_1m, output_cost_per_1m FROM model_pricing WHERE active = true;
```

Expected: 10 models (gpt-4o-mini, gpt-4o, claude-3-5-sonnet, etc.)

### Test 2: Backend API Endpoints

```bash
# Health check
curl http://localhost:3000/health

# Get model pricing (should return 10 models)
curl http://localhost:3000/api/v1/usage/pricing | jq

# Check usage for test user (replace with actual user_id)
curl http://localhost:3000/api/v1/usage/YOUR_USER_ID | jq
```

### Test 3: Cost Calculation Accuracy

Create a test script `test_cost_calculation.sql`:
```sql
-- Insert test user
INSERT INTO users (id, email, plan, monthly_token_limit, tokens_used_this_month, monthly_reset_at)
VALUES (
  'a1b2c3d4-e5f6-7890-abcd-ef1234567890'::uuid,
  'test@example.com',
  'pro',
  1000000,
  0,
  DATE_TRUNC('month', NOW() + INTERVAL '1 month')
);

-- GPT-4o-mini pricing: $0.15/1M input, $0.60/1M output
-- Test: 400 input + 900 output tokens = $0.00006 + $0.00054 = $0.00060
-- In INR: $0.00060 × 84 = ₹0.0504

SELECT 
  (400.0 / 1000000.0) * 0.15 AS input_cost,
  (900.0 / 1000000.0) * 0.60 AS output_cost,
  ((400.0 / 1000000.0) * 0.15) + ((900.0 / 1000000.0) * 0.60) AS total_cost_usd,
  (((400.0 / 1000000.0) * 0.15) + ((900.0 / 1000000.0) * 0.60)) * 84.0 AS total_cost_inr;
```

Expected output:
```
 input_cost | output_cost | total_cost_usd | total_cost_inr
------------+-------------+----------------+----------------
   0.00006  |   0.00054   |    0.00060     |     0.0504
```

### Test 4: Token Limit Enforcement

```sql
-- Set user to 95% of limit
UPDATE users 
SET tokens_used_this_month = 950000, 
    monthly_token_limit = 1000000
WHERE email = 'test@example.com';

-- Try to make a chat request via API
-- Should return warning but allow request

-- Set user to 100% of limit
UPDATE users 
SET tokens_used_this_month = 1000000
WHERE email = 'test@example.com';

-- Try to make another request
-- Should BLOCK with error message
```

### Test 5: Model Routing

```bash
# Test routing for free user (should always get gpt-4o-mini)
# Test routing for pro user with code task (should get claude-3-5-sonnet)
# Test routing for user at 95% usage (should downgrade to gpt-4o-mini)
```

### Test 6: Frontend Dashboard

1. Open Ghost desktop app
2. Click Settings (⚙️ icon)
3. Scroll to "Usage & Billing" section
4. Verify:
   - Plan badge shows correct plan
   - Token usage bar displays correctly
   - Cost summary shows USD amount
   - Model breakdown appears if you've made requests
   - Recent activity shows last messages
   - Auto-refresh works (wait 30s, check values update)

## 🔧 Troubleshooting

### Issue: Migration fails

**Symptom:**
```
ERROR: relation "users" does not exist
```

**Solution:**
Make sure `001_initial_schema.sql` was run first:
```bash
psql $DATABASE_URL -f migrations/001_initial_schema.sql
```

### Issue: Backend compilation errors

**Symptom:**
```
error[E0433]: failed to resolve: use of undeclared crate or module `rust_decimal`
```

**Solution:**
Add missing dependencies to `Cargo.toml`:
```toml
[dependencies]
rust_decimal = "1.33"
```

### Issue: Frontend can't fetch usage data

**Symptom:**
Dashboard shows "Failed to load usage data"

**Potential causes:**
1. Backend not running → Start with `cargo run`
2. Wrong API URL → Check `VITE_API_URL` in `.env`
3. No user_id → Pass actual user_id to UsageDashboard component
4. CORS issue → Check backend CORS configuration

**Debug:**
```bash
# Check backend logs
tail -f scribe-api/logs/app.log

# Check browser console
# Open DevTools → Console → Look for network errors
```

### Issue: Token usage not recording

**Symptom:**
Chat works but tokens_used_this_month doesn't increase

**Potential causes:**
1. User_id not found (no license_key header)
2. Usage recording failed but chat continued (graceful degradation)
3. Token extraction from stream failed

**Debug:**
Check backend logs for:
```
📊 Usage captured: prompt=X, completion=Y, total=Z
✅ Usage recorded successfully
```

If missing, check:
```bash
# Verify license_key header is sent
curl -H "x-license-key: YOUR_KEY" http://localhost:3000/api/v1/chat
```

## 📊 Monitoring

### Key Metrics to Watch

1. **Usage Recording Rate**
   ```sql
   SELECT COUNT(*) as total_messages, 
          SUM(total_tokens) as total_tokens,
          AVG(cost_usd) as avg_cost_per_message
   FROM messages 
   WHERE created_at > NOW() - INTERVAL '1 day';
   ```

2. **Users Near Limit**
   ```sql
   SELECT email, plan, 
          tokens_used_this_month, 
          monthly_token_limit,
          ROUND((tokens_used_this_month::float / monthly_token_limit::float) * 100, 2) as percentage_used
   FROM users 
   WHERE (tokens_used_this_month::float / monthly_token_limit::float) > 0.9
   ORDER BY percentage_used DESC;
   ```

3. **Most Expensive Users**
   ```sql
   SELECT u.email, mu.total_cost_usd, mu.total_tokens, mu.total_requests
   FROM monthly_usage mu
   JOIN users u ON mu.user_id = u.id
   WHERE mu.month = TO_CHAR(NOW(), 'YYYY-MM')
   ORDER BY mu.total_cost_usd DESC
   LIMIT 10;
   ```

4. **Model Usage Distribution**
   ```sql
   SELECT model, provider, 
          COUNT(*) as requests, 
          SUM(total_tokens) as total_tokens,
          SUM(cost_usd) as total_cost
   FROM messages
   WHERE created_at > NOW() - INTERVAL '7 days'
   GROUP BY model, provider
   ORDER BY total_cost DESC;
   ```

## 🎯 Success Criteria

✅ **Database:**
- [ ] Migration completes without errors
- [ ] All 4 tables exist (messages, monthly_usage, model_pricing, updated users)
- [ ] Trigger `reset_monthly_tokens` exists
- [ ] 10 models seeded in `model_pricing`

✅ **Backend:**
- [ ] Cargo build succeeds
- [ ] Server starts on port 3000
- [ ] All 4 usage endpoints respond (200 OK)
- [ ] Chat endpoint records usage
- [ ] Logs show "Usage recorded successfully"

✅ **Frontend:**
- [ ] npm build succeeds
- [ ] UsageDashboard component renders
- [ ] Dashboard shows usage data (not error state)
- [ ] Progress bar displays correctly
- [ ] Cost values are accurate

✅ **Integration:**
- [ ] Chat request increments tokens_used_this_month
- [ ] Cost calculation matches expected formula
- [ ] Token limit blocks requests at 100%
- [ ] Model routing works per plan tier
- [ ] Dashboard auto-refreshes every 30s

## 🚨 Known Issues

1. **user_id extraction:** Currently uses license_key header. You may need to update the header name based on your auth implementation.

2. **Monthly reset:** Trigger only runs on UPDATE. You may want a cron job:
   ```sql
   -- Run daily at midnight
   UPDATE users SET updated_at = NOW() WHERE monthly_reset_at < NOW();
   ```

3. **INR exchange rate:** Hardcoded to 84.0. Consider fetching from live API for accuracy.

4. **Streaming token extraction:** Assumes OpenAI-compatible format. May need adjustment for other providers.

## 📞 Support

If you encounter issues:

1. **Check logs:**
   - Backend: `scribe-api/logs/app.log`
   - Frontend: Browser DevTools → Console
   - Database: Check PostgreSQL logs

2. **Verify environment:**
   ```bash
   echo $DATABASE_URL
   cargo --version
   node --version
   ```

3. **Run tests:**
   ```bash
   cd scribe-api
   cargo test
   ```

---

**Quick deployment checklist:**
1. ✅ Run database migration
2. ✅ Build backend (cargo build --release)
3. ✅ Build frontend (npm run build)
4. ✅ Test all endpoints
5. ✅ Test frontend dashboard
6. ✅ Monitor for 24h
7. ✅ Deploy to production

**Estimated deployment time:** 30-60 minutes

---

*For detailed implementation details, see `USAGE_TRACKING_IMPLEMENTATION.md`*
