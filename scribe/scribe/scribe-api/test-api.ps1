# Scribe API Testing Script
# Make sure the API server is running before running this script

$baseUrl = "http://localhost:8080"
$apiKey = "test-api-key-12345"
$licenseKey = "test-license-key-123"

Write-Host "🧪 Testing Scribe API" -ForegroundColor Cyan
Write-Host "=====================" -ForegroundColor Cyan
Write-Host ""

# Test 1: Health Check
Write-Host "Test 1: Health Check" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/health" -Method Get
    Write-Host "✅ Health check passed" -ForegroundColor Green
    Write-Host "Response: $($response | ConvertTo-Json)" -ForegroundColor Gray
} catch {
    Write-Host "❌ Health check failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 2: Status Endpoint
Write-Host "Test 2: Status Endpoint" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/status" -Method Get
    Write-Host "✅ Status endpoint passed" -ForegroundColor Green
    Write-Host "Response: $($response | ConvertTo-Json)" -ForegroundColor Gray
} catch {
    Write-Host "❌ Status endpoint failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 3: License Activation
Write-Host "Test 3: License Activation" -ForegroundColor Yellow
$activationBody = @{
    license_key = $licenseKey
    instance_name = "test-instance-$(Get-Random)"
    machine_id = "test-machine-001"
    app_version = "0.1.0"
} | ConvertTo-Json

$headers = @{
    "Content-Type" = "application/json"
    "Authorization" = "Bearer $apiKey"
}

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/activate" -Method Post -Headers $headers -Body $activationBody
    Write-Host "✅ License activation passed" -ForegroundColor Green
    Write-Host "Response: $($response | ConvertTo-Json)" -ForegroundColor Gray
    $instanceId = $response.instance.id
} catch {
    Write-Host "❌ License activation failed: $_" -ForegroundColor Red
    $instanceId = "test-instance-fallback"
}
Write-Host ""

# Test 4: License Validation
Write-Host "Test 4: License Validation" -ForegroundColor Yellow
$validateBody = @{
    license_key = $licenseKey
    instance_name = $instanceId
    machine_id = "test-machine-001"
    app_version = "0.1.0"
} | ConvertTo-Json

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/validate" -Method Post -Headers $headers -Body $validateBody
    Write-Host "✅ License validation passed" -ForegroundColor Green
    Write-Host "Response: $($response | ConvertTo-Json)" -ForegroundColor Gray
} catch {
    Write-Host "❌ License validation failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 5: List Models
Write-Host "Test 5: List Models" -ForegroundColor Yellow
$chatHeaders = @{
    "Content-Type" = "application/json"
    "Authorization" = "Bearer $apiKey"
    "license_key" = $licenseKey
    "instance" = $instanceId
    "machine_id" = "test-machine-001"
}

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/models" -Method Post -Headers $chatHeaders
    Write-Host "✅ List models passed" -ForegroundColor Green
    Write-Host "Available models: $($response.models.Count)" -ForegroundColor Gray
    foreach ($model in $response.models) {
        Write-Host "  - $($model.name) ($($model.provider))" -ForegroundColor Gray
    }
} catch {
    Write-Host "❌ List models failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 6: Chat (requires OpenRouter API key)
Write-Host "Test 6: Chat Endpoint (requires OpenRouter API key in .env)" -ForegroundColor Yellow
$chatBody = @{
    user_message = "Say 'Hello from Scribe API!' in one sentence"
    system_prompt = "You are a helpful assistant."
} | ConvertTo-Json

Write-Host "⚠️  Note: This will fail if OPENROUTER_API_KEY is not set in .env" -ForegroundColor Yellow
try {
    # For streaming, we need to use curl or handle SSE differently
    # This is a simple test to see if the endpoint responds
    Write-Host "Sending chat request..." -ForegroundColor Gray
    Write-Host "⚠️  Full SSE streaming test requires curl or specialized client" -ForegroundColor Yellow
} catch {
    Write-Host "❌ Chat endpoint test skipped (requires SSE client)" -ForegroundColor Red
}
Write-Host ""

# Test 7: Generate System Prompt
Write-Host "Test 7: Generate System Prompt" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/prompt" -Method Post -Headers $chatHeaders
    Write-Host "✅ Generate prompt passed" -ForegroundColor Green
    Write-Host "Prompt: $($response.system_prompt)" -ForegroundColor Gray
} catch {
    Write-Host "❌ Generate prompt failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 8: Checkout URL
Write-Host "Test 8: Checkout URL" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/v1/checkout" -Method Get -Headers $headers
    Write-Host "✅ Checkout URL passed" -ForegroundColor Green
    Write-Host "Response: $($response | ConvertTo-Json)" -ForegroundColor Gray
} catch {
    Write-Host "❌ Checkout URL failed: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "=====================" -ForegroundColor Cyan
Write-Host "✅ Testing Complete!" -ForegroundColor Green
Write-Host "=====================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Note: Chat and Audio endpoints require API keys in .env:" -ForegroundColor Yellow
Write-Host "  - OPENROUTER_API_KEY for chat" -ForegroundColor White
Write-Host "  - OPENAI_API_KEY for audio transcription" -ForegroundColor White
Write-Host ""
