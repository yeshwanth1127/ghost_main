# Local Setup Guide - Scribe API

## Step-by-Step Setup on Windows

### 1. Install PostgreSQL

Download and install PostgreSQL 15+:
- Visit: https://www.postgresql.org/download/windows/
- Download the installer
- Run installer (remember the password you set for postgres user!)
- Default port: 5432

After installation, verify:
```powershell
psql --version
```

### 2. Install Redis (Optional but recommended)

Download Redis for Windows:
- Visit: https://github.com/microsoftarchive/redis/releases
- Download Redis-x64-3.0.504.msi
- Run installer
- Default port: 6379

Or use WSL2:
```bash
wsl
sudo apt install redis-server
redis-server
```

### 3. Create Database

Open PowerShell as Administrator:

```powershell
# Connect to PostgreSQL
psql -U postgres

# In the PostgreSQL prompt:
CREATE DATABASE scribe_db;
CREATE USER scribe_user WITH PASSWORD 'dev_password_123';
GRANT ALL PRIVILEGES ON DATABASE scribe_db TO scribe_user;
\q
```

Verify database was created:
```powershell
psql -U postgres -c "\l"
```

### 4. Setup Environment File

In `Scribe/scribe-api/`, create `.env`:

```env
# Server Configuration
HOST=0.0.0.0
PORT=8080
RUST_LOG=scribe_api=debug,info

# Database
DATABASE_URL=postgresql://scribe_user:dev_password_123@localhost:5432/scribe_db

# Redis
REDIS_URL=redis://localhost:6379

# API Security
API_ACCESS_KEY=test-api-key-12345

# OpenRouter Configuration (GET YOUR KEY FROM https://openrouter.ai/)
OPENROUTER_API_KEY=your-openrouter-key-here
OPENROUTER_BASE_URL=https://openrouter.ai/api/v1

# OpenAI (for Whisper STT) (GET FROM https://platform.openai.com/api-keys)
OPENAI_API_KEY=your-openai-key-here

# Payment Provider (Placeholder - not needed for testing)
PAYMENT_PROVIDER=stripe
STRIPE_API_KEY=sk_test_placeholder
STRIPE_WEBHOOK_SECRET=whsec_placeholder

# Monitoring (Optional)
SENTRY_DSN=
```

### 5. Install SQLx CLI

```powershell
cargo install sqlx-cli --no-default-features --features postgres
```

### 6. Run Database Migrations

```powershell
cd Scribe/scribe-api
sqlx migrate run
```

This will create all the tables (users, licenses, license_instances, usage_logs, transactions).

### 7. Verify Tables Were Created

```powershell
psql -U scribe_user -d scribe_db -c "\dt"
```

You should see:
- users
- licenses
- license_instances
- usage_logs
- transactions

### 8. Insert Test License (Optional)

```powershell
psql -U scribe_user -d scribe_db
```

```sql
INSERT INTO licenses (license_key, status, tier, max_instances)
VALUES ('test-license-key-123', 'active', 'pro', 5);

-- Verify
SELECT * FROM licenses;
\q
```

### 9. Build and Run API

```powershell
cd Scribe/scribe-api
cargo build --release
cargo run --release
```

Server should start on `http://localhost:8080`

### 10. Test Endpoints

Open a new PowerShell window:

```powershell
# Test health endpoint
curl http://localhost:8080/health

# Test status endpoint
curl http://localhost:8080/api/v1/status

# Test license activation
curl -X POST http://localhost:8080/api/v1/activate `
  -H "Content-Type: application/json" `
  -H "Authorization: Bearer test-api-key-12345" `
  -d '{\"license_key\":\"test-license-key-123\",\"instance_name\":\"test-instance\",\"machine_id\":\"test-machine-001\",\"app_version\":\"0.1.0\"}'

# Test chat (requires OpenRouter API key)
curl -X POST http://localhost:8080/api/v1/chat `
  -H "Content-Type: application/json" `
  -H "Authorization: Bearer test-api-key-12345" `
  -H "license_key: test-license-key-123" `
  -H "instance: test-instance" `
  -H "machine_id: test-machine-001" `
  -d '{\"user_message\":\"Hello, how are you?\",\"system_prompt\":\"You are a helpful assistant.\"}'
```

## Troubleshooting

### Database Connection Failed
```powershell
# Check if PostgreSQL is running
Get-Service -Name postgresql*

# Start if not running
Start-Service postgresql-x64-15
```

### Port Already in Use
```powershell
# Check what's using port 8080
netstat -ano | findstr :8080

# Kill the process (replace PID)
taskkill /PID <PID> /F
```

### Migration Failed
```powershell
# Reset database
psql -U postgres -c "DROP DATABASE scribe_db;"
psql -U postgres -c "CREATE DATABASE scribe_db;"
sqlx migrate run
```

### API Keys Not Found
Make sure `.env` file is in `Scribe/scribe-api/` directory and contains all required keys.

## Next Steps

Once local testing works:
1. Get OpenRouter API key: https://openrouter.ai/
2. Get OpenAI API key: https://platform.openai.com/api-keys
3. Update `.env` with real keys
4. Test chat and audio endpoints with real keys
5. Update Tauri desktop app `.env` to point to local API
6. Test end-to-end flow
