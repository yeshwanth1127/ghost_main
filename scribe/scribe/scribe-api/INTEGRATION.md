# Scribe API Integration Guide

## Frontend Integration (Tauri Desktop App)

The existing Tauri code in `Scribe/src-tauri/src/api.rs` and `Scribe/src-tauri/src/activate.rs` is **already compatible** with this API. You only need to update environment variables.

### Step 1: Update Environment Variables

Create or update `Scribe/src-tauri/.env`:

```env
APP_ENDPOINT=https://api.scribe.com
PAYMENT_ENDPOINT=https://api.scribe.com/api/v1
API_ACCESS_KEY=your-api-server-access-key
```

### Step 2: Build Configuration

The Tauri app will compile these environment variables at build time. Ensure they're set before building:

```bash
cd Scribe
npm run tauri build
```

### Step 3: No Code Changes Required

The following endpoints are already implemented in the Tauri app:

**License Management** (`src-tauri/src/activate.rs`):
- ✅ POST `/api/v1/activate` - Already calls this
- ✅ POST `/api/v1/deactivate` - Already calls this
- ✅ POST `/api/v1/validate` - Already calls this
- ✅ GET `/api/v1/checkout` - Already calls this

**AI Services** (`src-tauri/src/api.rs`):
- ✅ POST `/api/v1/chat` - Already calls this with streaming
- ✅ POST `/api/v1/audio` - Already calls this
- ✅ POST `/api/v1/models` - Already calls this
- ✅ POST `/api/v1/prompt` - Already calls this

## API Request Format

### Chat Request (Streaming)

```rust
// Tauri already sends this format
ChatRequest {
    user_message: String,
    system_prompt: Option<String>,
    image_base64: Option<Value>, // String or Vec<String>
    history: Option<String>,      // JSON array of messages
}
```

**Headers**:
```
Authorization: Bearer {API_ACCESS_KEY}
license_key: {user_license_key}
instance: {instance_id}
machine_id: {machine_id}
provider: {selected_provider} // Optional
model: {selected_model}       // Optional
```

**Response**: Server-Sent Events (SSE)
```
data: {"choices":[{"delta":{"content":"Hello"}}]}

data: {"choices":[{"delta":{"content":" world"}}]}

data: [DONE]
```

### Audio/STT Request

```rust
AudioRequest {
    audio_base64: String,
}
```

**Response**:
```json
{
  "success": true,
  "transcription": "Hello, this is a test.",
  "error": null
}
```

## How It Works

### 1. License Activation Flow

```
Scribe Desktop App
    ↓ (user enters license key)
POST /api/v1/activate
    ↓ (license_key, instance_name, machine_id, app_version)
Scribe API validates license
    ↓ creates license_instance record
Returns activation response
    ↓ (activated: true, instance_id)
Desktop stores in secure_storage.json
```

### 2. AI Chat Flow

```
Scribe Desktop App
    ↓ (user_message + optional images)
POST /api/v1/chat with headers (license_key, instance, machine_id)
    ↓
Scribe API validates license (middleware)
    ↓
Selects appropriate model based on input
    ↓ (text-only → Claude 3.5 Sonnet, images → Vision model)
Routes to OpenRouter
    ↓
Streams response back via SSE
    ↓
Desktop displays chunks in real-time
```

### 3. Speech-to-Text Flow

```
Scribe Desktop App
    ↓ (records audio, converts to base64)
POST /api/v1/audio
    ↓ (audio_base64, license_key, instance, machine_id)
Scribe API validates license
    ↓
Sends to OpenAI Whisper
    ↓
Returns transcription
    ↓
Desktop displays text
```

## Model Selection Strategy

The API automatically selects the best model based on input type:

| Input Type | Selected Model | Use Case |
|------------|---------------|----------|
| Text only | `anthropic/claude-3.5-sonnet` | General chat, fast responses |
| Text + Images | `anthropic/claude-3.5-sonnet` | Vision tasks, screenshot analysis |
| Need speed | `anthropic/claude-3-haiku` | Quick responses, real-time |
| Custom | User's choice via `provider` header | Advanced users |

All models are routed through **OpenRouter** which provides:
- Access to 100+ models
- Automatic fallback
- Cost optimization
- Usage tracking

## Testing the Integration

### 1. Test Health Endpoint
```bash
curl https://api.scribe.com/health
```

Expected response:
```json
{
  "status": "ok",
  "service": "scribe-api"
}
```

### 2. Test License Activation
```bash
curl -X POST https://api.scribe.com/api/v1/activate \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-access-key" \
  -d '{
    "license_key": "test-license-key",
    "instance_name": "test-instance",
    "machine_id": "test-machine",
    "app_version": "0.1.7"
  }'
```

### 3. Test Chat (with license)
```bash
curl -X POST https://api.scribe.com/api/v1/chat \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer your-api-access-key" \
  -H "license_key: test-license-key" \
  -H "instance: instance-id" \
  -H "machine_id: test-machine" \
  -d '{
    "user_message": "Hello, how are you?",
    "system_prompt": "You are a helpful assistant."
  }'
```

## Deployment Checklist

- [ ] PostgreSQL database created
- [ ] Redis installed and running
- [ ] Environment variables configured
- [ ] Database migrations run (`sqlx migrate run`)
- [ ] OpenRouter API key configured
- [ ] OpenAI API key configured (for Whisper)
- [ ] API_ACCESS_KEY set (for frontend auth)
- [ ] Nginx reverse proxy configured
- [ ] SSL certificate installed
- [ ] Service running (`systemctl status scribe-api`)
- [ ] Health endpoint accessible
- [ ] Test license activation works
- [ ] Test chat endpoint works
- [ ] Test audio endpoint works

## Troubleshooting

### License Validation Fails
- Check `license_key`, `instance`, `machine_id` headers are sent
- Verify license exists in database and status = 'active'
- Check license hasn't expired

### Chat Streaming Not Working
- Ensure client supports SSE (Tauri's reqwest does)
- Check OpenRouter API key is valid
- Verify nginx has `proxy_buffering off`

### Audio Transcription Fails
- Verify audio is base64 encoded WAV format
- Check OpenAI API key is valid
- Ensure audio file size < 25MB

## Next Steps

1. Deploy API to VPS using `deploy/setup-vps.sh`
2. Configure domain and SSL
3. Update Tauri `.env` with production API endpoint
4. Build and distribute desktop app
5. Monitor logs and usage
