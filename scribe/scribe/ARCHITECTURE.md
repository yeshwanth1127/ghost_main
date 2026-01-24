# Ghost (Scribe) - Complete System Architecture

## 1. System Overview

**Ghost** is a privacy-first AI desktop assistant built with Tauri (Rust + React/TypeScript). It provides:
- Real-time system audio capture and transcription
- AI-powered chat with multiple provider support
- Screen capture and image analysis
- Action execution system (file operations, etc.)
- License management and trial system
- Cross-platform support (macOS, Windows, Linux)

---

## 2. Technology Stack

### Frontend
- **Framework**: React 19.1.0 + TypeScript
- **Build Tool**: Vite 7.0.4
- **UI Library**: Radix UI + Tailwind CSS 4.1.12
- **State Management**: React Context API
- **Audio Processing**: `@ricky0123/vad-react` (Voice Activity Detection)

### Backend (Tauri)
- **Framework**: Tauri 2.5.2
- **Language**: Rust (Edition 2021)
- **Async Runtime**: Tokio
- **Audio Capture**: Platform-specific libraries
  - macOS: `cidre` + `tauri-nspanel`
  - Windows: `wasapi`
  - Linux: `libpulse-binding`
- **Screen Capture**: `xcap`
- **Database**: SQLite via `tauri-plugin-sql`

### API Server (scribe-api)
- **Framework**: Axum (Rust)
- **Database**: PostgreSQL (via SQLx)
- **HTTP Client**: Reqwest
- **Streaming**: Server-Sent Events (SSE)
- **Services**: OpenRouter, Whisper (OpenAI)

---

## 3. Architecture Layers

```
┌─────────────────────────────────────────────────────────┐
│                    Frontend (React)                      │
│  Components, Hooks, Contexts, UI State Management       │
└────────────────────┬────────────────────────────────────┘
                      │ Tauri Commands (invoke)
┌─────────────────────▼────────────────────────────────────┐
│              Tauri Backend (Rust)                        │
│  Commands, Event Emitters, Platform APIs, State        │
└────────────────────┬────────────────────────────────────┘
                      │ HTTP Requests
┌─────────────────────▼────────────────────────────────────┐
│            Scribe API Server (Axum)                      │
│  Routes, Services, Database, External APIs              │
└──────────────────────────────────────────────────────────┘
```

---

## 4. Data Flow

### 4.1 Audio Capture & Transcription Flow

```
1. User starts system audio capture
   ↓
2. Frontend: useSystemAudio hook calls `start_system_audio_capture`
   ↓
3. Tauri: Platform-specific audio capture starts (macOS/Windows/Linux)
   ↓
4. VAD (Voice Activity Detection) processes audio stream:
   - Detects speech start/end
   - Applies noise gate
   - Normalizes audio levels
   ↓
5. Tauri emits "speech-detected" event with base64 WAV audio
   ↓
6. Frontend receives event, converts to Blob
   ↓
7. Frontend calls `transcribe_audio` Tauri command
   ↓
8. Tauri sends POST to `/api/v1/audio` with:
   - Headers: license_key, instance, machine_id, API_ACCESS_KEY
   - Body: { audio_base64: string }
   ↓
9. API Server:
   - Validates license
   - Calls WhisperService (OpenAI Whisper API)
   - Returns transcription
   ↓
10. Frontend receives transcription
   ↓
11. Transcription is sent to AI chat (if auto-submit enabled)
```

### 4.2 Chat Flow

```
1. User submits message (text or transcribed audio)
   ↓
2. Frontend: useCompletion hook calls `chat_stream`
   ↓
3. Tauri: `api::chat_stream` command:
   - Gets stored credentials (license_key, instance_id, model)
   - Gets machine_id
   - Prepares ChatRequest
   ↓
4. Tauri sends POST to `/api/v1/chat?stream=true`:
   - Headers: Authorization, license_key, instance, provider, model, machine_id
   - Body: { user_message, system_prompt?, image_base64?, history? }
   ↓
5. API Server:
   - Validates license
   - Calls OpenRouterService with model_id
   - Returns SSE stream
   ↓
6. Tauri processes SSE stream:
   - Parses JSON chunks
   - Extracts text content (supports multiple formats)
   - Emits "chat_stream_chunk" events
   ↓
7. Frontend receives chunks via event listener
   ↓
8. Frontend updates UI with streaming text
   ↓
9. On completion, emits "chat_stream_complete" with full response
   ↓
10. Frontend saves to SQLite database (conversations/messages tables)
```

### 4.3 Screen Capture Flow

```
1. User triggers screenshot shortcut
   ↓
2. Frontend calls `start_screen_capture` Tauri command
   ↓
3. Tauri:
   - Captures full screen using xcap
   - Stores image in CaptureState
   - Creates overlay window (fullscreen, transparent)
   ↓
4. Overlay component renders selection UI
   ↓
5. User selects area
   ↓
6. Frontend calls `capture_selected_area` with coordinates
   ↓
7. Tauri:
   - Crops stored image to selection
   - Encodes to PNG base64
   - Emits "captured-selection" event
   ↓
8. Frontend receives base64 image
   ↓
9. Image can be:
   - Sent to AI chat for analysis
   - Saved locally
```

### 4.4 License Activation Flow

```
1. User enters license key
   ↓
2. Frontend calls `activate_license_api` Tauri command
   ↓
3. Tauri:
   - Gets machine_id
   - Generates instance_id (UUID)
   - Sends POST to `/api/v1/activate`
   ↓
4. API Server:
   - Validates license_key exists and is active
   - Creates license_instance record
   - Returns activation response
   ↓
5. Tauri stores credentials in secure storage:
   - license_key
   - instance_id
   - selected_model (optional)
   ↓
6. Frontend updates license status
```

---

## 5. Key Components

### 5.1 Frontend Components

#### Core Components
- **App.tsx**: Main application component, orchestrates UI
- **Overlay.tsx**: Screen capture selection overlay
- **Completion**: Chat completion UI with streaming
- **ChatHistory**: Conversation history sidebar
- **Settings**: Configuration UI
- **AudioVisualizer**: Real-time audio visualization

#### Hooks
- **useSystemAudio**: Manages system audio capture state
- **useCompletion**: Handles chat streaming and state
- **useActionAssistant**: Manages action planning/execution
- **useApp**: Main app state hook

#### Contexts
- **AppProvider**: Global state (providers, settings, license)
- **ThemeProvider**: Theme management

### 5.2 Tauri Backend Modules

#### Core Modules
- **lib.rs**: Application entry point, plugin setup
- **api.rs**: HTTP API communication (chat, audio, models)
- **activate.rs**: License management
- **speaker/mod.rs**: Platform-agnostic audio capture interface
- **speaker/commands.rs**: Audio capture commands, VAD logic
- **capture.rs**: Screen capture functionality
- **shortcuts.rs**: Global keyboard shortcuts
- **window.rs**: Window management
- **db/mod.rs**: SQLite migrations

#### Assistant Module
- **assistant/commands.rs**: Action planning/execution commands
- **assistant/planner/**: Intent parsing, plan verification
- **assistant/executor/**: Action execution worker
- **assistant/policy.rs**: Capability tokens
- **assistant/audit.rs**: Audit logging

### 5.3 API Server (scribe-api)

#### Routes
- `/api/v1/chat`: Streaming chat endpoint
- `/api/v1/audio`: Audio transcription
- `/api/v1/models`: List available models
- `/api/v1/prompt`: Generate system prompts
- `/api/v1/activate`: License activation
- `/api/v1/validate`: License validation
- `/api/v1/create-trial`: Create trial license
- `/api/v1/leave-applications`: Submit leave applications

#### Services
- **OpenRouterService**: LLM API integration
- **WhisperService**: Speech-to-text via OpenAI
- **LicenseService**: License management
- **LeaveService**: Leave application handling

#### Middleware
- **auth.rs**: License validation middleware
- **log_request**: Request logging

---

## 6. Database Schema

### 6.1 SQLite (Desktop App)

#### system_prompts
```sql
CREATE TABLE system_prompts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    prompt TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### conversations
```sql
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    title TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### messages
```sql
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id)
);
```

#### action_snapshots
```sql
CREATE TABLE action_snapshots (
    id TEXT PRIMARY KEY,
    action_plan TEXT NOT NULL,
    executed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### audit_logs
```sql
CREATE TABLE audit_logs (
    id TEXT PRIMARY KEY,
    action_id TEXT NOT NULL,
    action_type TEXT NOT NULL,
    details TEXT,
    executed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 6.2 PostgreSQL (API Server)

#### licenses
```sql
CREATE TABLE licenses (
    id UUID PRIMARY KEY,
    license_key TEXT UNIQUE NOT NULL,
    user_id UUID,
    status TEXT, -- active, suspended, expired
    tier TEXT, -- trial, free, basic, pro, enterprise
    max_instances INTEGER,
    is_trial BOOLEAN,
    trial_ends_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### license_instances
```sql
CREATE TABLE license_instances (
    id UUID PRIMARY KEY,
    license_id UUID REFERENCES licenses(id),
    instance_name TEXT NOT NULL,
    machine_id TEXT NOT NULL,
    app_version TEXT,
    last_validated_at TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(license_id, machine_id)
);
```

#### usage_logs
```sql
CREATE TABLE usage_logs (
    id UUID PRIMARY KEY,
    license_id UUID REFERENCES licenses(id),
    endpoint TEXT,
    model_used TEXT,
    tokens_used INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

---

## 7. Security & Authentication

### 7.1 License System
- **License Key**: Stored in secure storage (OS keychain)
- **Instance ID**: UUID per machine
- **Machine ID**: Hardware-based identifier
- **Validation**: Periodic checks against API server
- **Trial System**: Time-limited trial licenses

### 7.2 API Authentication
- **API_ACCESS_KEY**: Bearer token for API server
- **License Key**: Header-based validation
- **Machine ID**: Device fingerprinting
- **Instance ID**: Multi-device support

### 7.3 Secure Storage
- Uses OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Stores: license_key, instance_id, selected_model

---

## 8. Platform-Specific Features

### macOS
- **NSPanel**: Floating panel window
- **macOS Private API**: Enhanced window behavior
- **Audio Capture**: CoreAudio via `cidre`
- **Permissions**: macOS permissions plugin

### Windows
- **WASAPI**: Audio capture
- **Window Management**: Special handling for Windows

### Linux
- **PulseAudio**: Audio capture
- **Desktop Integration**: `.desktop` file support

---

## 9. Configuration & Environment

### Environment Variables (Tauri)
- `APP_ENDPOINT`: API server URL
- `API_ACCESS_KEY`: API authentication key
- `PAYMENT_ENDPOINT`: Payment/license endpoint
- `POSTHOG_API_KEY`: Analytics (optional)

### Environment Variables (API Server)
- `DATABASE_URL`: PostgreSQL connection
- `LEAVE_DATABASE_URL`: Leave applications DB
- `OPENAI_API_KEY`: Whisper API key
- `OPENROUTER_API_KEY`: OpenRouter API key
- `API_ACCESS_KEY`: Server authentication key

---

## 10. Build & Deployment

### Desktop App Build
```bash
npm run build          # Build frontend
npm run tauri build    # Build Tauri app
```

**Output**: `src-tauri/target/release/bundle/`
- `deb/`: Debian packages
- `rpm/`: RPM packages
- `appimage/`: AppImage files
- `macos/`: .dmg files (on macOS)
- `msi/` or `nsis/`: Windows installers (on Windows)

### API Server Build
```bash
cd scribe-api
cargo build --release
```

**Deployment**: Systemd service or Docker container

---

## 11. Key Features

### 11.1 Audio Features
- **System Audio Capture**: Captures speaker output (meetings, calls)
- **VAD (Voice Activity Detection)**: Automatic speech detection
- **Noise Gating**: Filters background noise
- **Continuous Recording**: Manual recording mode
- **Real-time Visualization**: Audio waveform display

### 11.2 AI Features
- **Multi-Provider Support**: OpenRouter, custom providers
- **Streaming Responses**: Real-time text streaming
- **Image Analysis**: Screenshot analysis
- **System Prompt Management**: Customizable prompts
- **Chat History**: Persistent conversation storage

### 11.3 Action System
- **Intent Parsing**: Natural language to actions
- **LLM Planning**: AI-generated action plans
- **Plan Verification**: Safety checks before execution
- **Action Execution**: File operations, etc.
- **Audit Logging**: Action history tracking
- **Undo Support**: Reversible actions

### 11.4 UI Features
- **Global Shortcuts**: Keyboard shortcuts
- **Window Management**: Hide/show, always-on-top
- **Customizable UI**: Cursor, window behavior
- **Auto-start**: Launch on system boot
- **Update System**: Built-in updater

---

## 12. Data Flow Diagrams

### Audio → Transcription → Chat
```
System Audio → VAD → WAV Base64 → API → Whisper → Transcription → AI Chat
```

### Chat Request Flow
```
User Input → Frontend → Tauri Command → API Server → OpenRouter → LLM → SSE Stream → Frontend → UI Update
```

### License Validation Flow
```
App Start → Check Secure Storage → Validate License API → Update UI State
```

---

## 13. Error Handling

### Frontend
- Try-catch blocks in async functions
- Error state management in hooks
- User-friendly error messages
- Retry logic for network requests

### Tauri
- Result types for all commands
- Error propagation via String errors
- Event-based error notifications
- Graceful degradation

### API Server
- HTTP status codes
- JSON error responses
- Structured error logging
- Database transaction rollback

---

## 14. Performance Optimizations

### Audio Processing
- Chunked processing (hop_size)
- Pre-allocated buffers
- Noise gate before VAD
- Audio normalization

### Frontend
- React memoization
- Lazy loading
- Event debouncing
- Efficient state updates

### API
- Connection pooling
- Streaming responses
- Async/await throughout
- Database indexing

---

## 15. Testing & Development

### Development
```bash
npm run dev          # Start dev server
npm run tauri dev    # Run Tauri app in dev mode
```

### API Server
```bash
cd scribe-api
cargo run            # Run API server
```

### Database Migrations
- Tauri: SQL migrations in `src-tauri/src/db/migrations/`
- API: SQLx migrations in `scribe-api/migrations/`

---

## 16. Future Enhancements

Potential areas for expansion:
- Multi-language support
- Plugin system
- Cloud sync
- Advanced action types
- Team collaboration features
- Enhanced analytics

---

## 17. File Structure Summary

```
scribe/
├── src/                          # Frontend React app
│   ├── components/              # UI components
│   ├── hooks/                    # React hooks
│   ├── contexts/                 # Context providers
│   ├── lib/                      # Utilities, functions
│   └── types/                    # TypeScript types
├── src-tauri/                    # Tauri backend
│   ├── src/
│   │   ├── api.rs               # API communication
│   │   ├── activate.rs           # License management
│   │   ├── speaker/              # Audio capture
│   │   ├── assistant/            # Action system
│   │   ├── capture.rs            # Screen capture
│   │   └── db/                   # Database migrations
│   └── Cargo.toml
├── scribe-api/                   # API server
│   ├── src/
│   │   ├── routes/               # API endpoints
│   │   ├── services/              # Business logic
│   │   ├── models/                # Data models
│   │   └── db/                    # Database setup
│   └── migrations/               # SQL migrations
└── package.json                  # Frontend dependencies
```

---

This architecture document provides a comprehensive overview of the Ghost (Scribe) system. For specific implementation details, refer to the source code in each module.

