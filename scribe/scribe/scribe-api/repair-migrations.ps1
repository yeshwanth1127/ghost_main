# Repair orphaned migration version in _sqlx_migrations
# Run this if you get: VersionMissing(6) or similar errors
# Usage: .\repair-migrations.ps1 [version_number]
# Example: .\repair-migrations.ps1 6

param(
    [int]$Version = 6
)

$envFile = ".env"
if (-not (Test-Path $envFile)) {
    Write-Host "Error: .env not found. Run from scribe-api directory." -ForegroundColor Red
    exit 1
}

$dbUrl = (Get-Content $envFile | Where-Object { $_ -match "^DATABASE_URL=" } | ForEach-Object { $_ -replace "^DATABASE_URL=", "" }).Trim().Trim('"').Trim("'")
if (-not $dbUrl) {
    Write-Host "Error: DATABASE_URL not found in .env" -ForegroundColor Red
    exit 1
}

Write-Host "Removing orphaned migration version $Version from _sqlx_migrations..." -ForegroundColor Yellow
$result = psql $dbUrl -c "DELETE FROM _sqlx_migrations WHERE version = $Version;"
if ($LASTEXITCODE -eq 0) {
    Write-Host "Done. You can now run: cargo run" -ForegroundColor Green
} else {
    Write-Host "Failed. Try running manually: psql `$DATABASE_URL -c `"DELETE FROM _sqlx_migrations WHERE version = $Version;`"" -ForegroundColor Red
    exit 1
}
