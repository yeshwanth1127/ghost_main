# Scribe API Local Setup Script for Windows
# Run this in PowerShell as Administrator

Write-Host "🚀 Scribe API Local Setup" -ForegroundColor Cyan
Write-Host "=========================" -ForegroundColor Cyan
Write-Host ""

# Check if running as administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "⚠️  Please run this script as Administrator!" -ForegroundColor Red
    exit 1
}

# Step 1: Check PostgreSQL
Write-Host "Step 1: Checking PostgreSQL..." -ForegroundColor Yellow
$pgService = Get-Service -Name postgresql* -ErrorAction SilentlyContinue
if ($pgService) {
    Write-Host "✅ PostgreSQL service found: $($pgService.Name)" -ForegroundColor Green
    if ($pgService.Status -ne "Running") {
        Write-Host "Starting PostgreSQL..." -ForegroundColor Yellow
        Start-Service $pgService.Name
    }
} else {
    Write-Host "❌ PostgreSQL not found. Please install from: https://www.postgresql.org/download/windows/" -ForegroundColor Red
    exit 1
}

# Step 2: Check Rust
Write-Host "`nStep 2: Checking Rust..." -ForegroundColor Yellow
$rustVersion = cargo --version 2>$null
if ($rustVersion) {
    Write-Host "✅ Rust installed: $rustVersion" -ForegroundColor Green
} else {
    Write-Host "❌ Rust not found. Please install from: https://rustup.rs/" -ForegroundColor Red
    exit 1
}

# Step 3: Create database
Write-Host "`nStep 3: Creating database..." -ForegroundColor Yellow
$createDbScript = @"
CREATE DATABASE scribe_db;
CREATE USER scribe_user WITH PASSWORD 'dev_password_123';
GRANT ALL PRIVILEGES ON DATABASE scribe_db TO scribe_user;
"@

$env:PGPASSWORD = Read-Host "Enter PostgreSQL 'postgres' user password" -AsSecureString | ConvertFrom-SecureString -AsPlainText
psql -U postgres -c "SELECT 1" 2>$null
if ($LASTEXITCODE -eq 0) {
    # Check if database already exists
    $dbExists = psql -U postgres -tAc "SELECT 1 FROM pg_database WHERE datname='scribe_db'" 2>$null
    if ($dbExists -eq "1") {
        Write-Host "⚠️  Database 'scribe_db' already exists" -ForegroundColor Yellow
        $recreate = Read-Host "Recreate database? (y/n)"
        if ($recreate -eq "y") {
            psql -U postgres -c "DROP DATABASE scribe_db;" 2>$null
            psql -U postgres -c $createDbScript
            Write-Host "✅ Database recreated" -ForegroundColor Green
        }
    } else {
        psql -U postgres -c $createDbScript
        Write-Host "✅ Database created" -ForegroundColor Green
    }
} else {
    Write-Host "❌ Failed to connect to PostgreSQL. Check password." -ForegroundColor Red
    exit 1
}

# Step 4: Check if .env exists
Write-Host "`nStep 4: Checking environment file..." -ForegroundColor Yellow
if (-not (Test-Path ".env")) {
    if (Test-Path "env.example") {
        Copy-Item "env.example" ".env"
        Write-Host "✅ Created .env from env.example" -ForegroundColor Green
        Write-Host "⚠️  Please edit .env and add your API keys!" -ForegroundColor Yellow
        Write-Host "   - OPENROUTER_API_KEY: Get from https://openrouter.ai/" -ForegroundColor Cyan
        Write-Host "   - OPENAI_API_KEY: Get from https://platform.openai.com/api-keys" -ForegroundColor Cyan
    } else {
        Write-Host "❌ env.example not found" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "✅ .env file exists" -ForegroundColor Green
}

# Step 5: Install sqlx-cli
Write-Host "`nStep 5: Checking sqlx-cli..." -ForegroundColor Yellow
$sqlxVersion = sqlx --version 2>$null
if ($sqlxVersion) {
    Write-Host "✅ sqlx-cli installed: $sqlxVersion" -ForegroundColor Green
} else {
    Write-Host "Installing sqlx-cli (this may take a few minutes)..." -ForegroundColor Yellow
    cargo install sqlx-cli --no-default-features --features postgres
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ sqlx-cli installed" -ForegroundColor Green
    } else {
        Write-Host "❌ Failed to install sqlx-cli" -ForegroundColor Red
        exit 1
    }
}

# Step 6: Run migrations
Write-Host "`nStep 6: Running database migrations..." -ForegroundColor Yellow
sqlx migrate run
if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Migrations completed" -ForegroundColor Green
} else {
    Write-Host "❌ Migrations failed" -ForegroundColor Red
    exit 1
}

# Step 7: Verify tables
Write-Host "`nStep 7: Verifying tables..." -ForegroundColor Yellow
$tables = psql -U scribe_user -d scribe_db -tAc "\dt" 2>$null
if ($tables) {
    Write-Host "✅ Tables created:" -ForegroundColor Green
    psql -U scribe_user -d scribe_db -c "\dt"
} else {
    Write-Host "❌ No tables found" -ForegroundColor Red
}

# Step 8: Insert test license
Write-Host "`nStep 8: Creating test license..." -ForegroundColor Yellow
$insertLicense = @"
INSERT INTO licenses (license_key, status, tier, max_instances)
VALUES ('test-license-key-123', 'active', 'pro', 5)
ON CONFLICT DO NOTHING;
"@
$env:PGPASSWORD = "dev_password_123"
psql -U scribe_user -d scribe_db -c $insertLicense 2>$null
if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Test license created: test-license-key-123" -ForegroundColor Green
} else {
    Write-Host "⚠️  License may already exist" -ForegroundColor Yellow
}

Write-Host "`n=========================" -ForegroundColor Cyan
Write-Host "✅ Setup Complete!" -ForegroundColor Green
Write-Host "=========================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Edit .env file and add your API keys" -ForegroundColor White
Write-Host "2. Run: cargo build --release" -ForegroundColor White
Write-Host "3. Run: cargo run --release" -ForegroundColor White
Write-Host "4. Test: curl http://localhost:8080/health" -ForegroundColor White
Write-Host ""
Write-Host "Test license key: test-license-key-123" -ForegroundColor Cyan
Write-Host ""
