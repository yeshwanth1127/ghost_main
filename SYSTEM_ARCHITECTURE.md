# Ghost (Scribe) - Complete System Architecture Documentation

## 📋 Table of Contents
1. [System Overview](#system-overview)
2. [Technology Stack](#technology-stack)
3. [System Architecture](#system-architecture)
4. [Module Breakdown](#module-breakdown)
5. [Data Flow Diagrams](#data-flow-diagrams)
6. [API Reference](#api-reference)
7. [Database Schema](#database-schema)
8. [Security & Authentication](#security--authentication)
9. [Platform-Specific Implementation](#platform-specific-implementation)

---

## 🎯 System Overview

**Ghost** is a privacy-first AI desktop assistant that provides real-time audio transcription, AI-powered chat, screen capture, and action execution capabilities. It's built as a cross-platform desktop application with a client-server architecture.

### Key Features
- **Real-time System Audio Capture**: Captures speaker output (meetings, calls) with Voice Activity Detection (VAD)
- **AI-Powered Chat**: Multi-provider LLM support with streaming responses
- **Screen Capture & Analysis**: Full-screen capture with area selection and image analysis
- **Action Execution System**: Natural language intent parsing and action execution
- **License Management**: Trial and subscription-based licensing system
- **Cross-Platform Support**: macOS, Windows, and Linux

---

## 🛠 Technology Stack

### Frontend (React/TypeScript)
```
Framework:        React 19.1.0 + TypeScript
Build Tool:       Vite 7.0.4
UI Library:       Radix UI + Tailwind CSS 4.1.12
State Management: React Context API
Audio Processing: @ricky0123/vad-react (Voice Activity Detection)
Markdown:         react-markdown with syntax highlighting (shiki)
Icons:            lucide-react
```

### Backend (Tauri/Rust)
```
Framework:        Tauri 2.5.2
Language:         Rust (Edition 2021)
Async Runtime:    Tokio
Audio Capture:
  - macOS:        cidre (CoreAudio) + tauri-nspanel
  - Windows:      wasapi
  - Linux:        libpulse-binding
Screen Capture:   xcap
Database:         SQLite via tauri-plugin-sql
HTTP Client:      reqwest (with streaming support)
```

### API Server (Axum/Rust)
```
Framework:        Axum 0.7
Database:         PostgreSQL (via SQLx)
HTTP Client:      Reqwest
Streaming:        Server-Sent Events (SSE)
Services:         OpenRouter (LLM), OpenAI Whisper (STT)
Caching:          Redis (ready for implementation)
```

---

## 🏗 System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend (React)                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ App.tsx  │  │ Components│  │  Hooks   │  │ Contexts │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  │
│       └─────────────┼──────────────┼──────────────┘        │
└─────────────────────┼──────────────┼────────────────────────┘
                      │ Tauri Commands (invoke)
                      │ Tauri Events (listen)
┌─────────────────────▼──────────────▼────────────────────────┐
│              Tauri Backend (Rust)                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │  api.rs  │  │ speaker/ │  │ capture  │  │assistant │  │
│  │ activate │  │  audio   │  │  screen  │  │  actions │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  │
│       └─────────────┼──────────────┼──────────────┘        │
└─────────────────────┼──────────────┼────────────────────────┘
                      │ HTTP/HTTPS
                      │ (REST API + SSE)
┌─────────────────────▼──────────────▼────────────────────────┐
│            Scribe API Server (Axum)                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ Routes   │  │ Services │  │ Models   │  │ Middleware│ │
│  │  /chat   │  │OpenRouter│  │License   │  │  Auth    │  │
│  │  /audio  │  │ Whisper  │  │ Instance │  │  CORS    │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  │
│       └─────────────┼──────────────┼──────────────┘        │
└─────────────────────┼──────────────┼────────────────────────┘
                      │              │
            ┌─────────▼────┐  ┌─────▼─────────┐
            │  PostgreSQL  │  │  OpenRouter   │
            │   Database   │  │  (LLM APIs)   │
            └──────────────┘  └───────────────┘
```

### Layer Responsibilities

1. **Frontend Layer**: UI rendering, user interaction, state management, event handling
2. **Tauri Layer**: Platform APIs, secure storage, local database, event bridge
3. **API Layer**: Business logic, license validation, external service integration
4. **Database Layer**: Data persistence (SQLite locally, PostgreSQL on server)
5. **External Services**: LLM providers, STT services

---

## 📦 Module Breakdown

### Frontend Modules (`src/`)

#### Core Components
- **`App.tsx`**: Main application orchestrator, manages window visibility, system audio state
- **`components/completion/`**: Chat completion UI with streaming support
- **`components/history/`**: Conversation history sidebar
- **`components/settings/`**: Configuration UI (providers, models, shortcuts, VAD)
- **`components/Overlay.tsx`**: Screen capture selection overlay
- **`components/speech/`**: Audio visualization and controls
- **`components/assistant/`**: Action planning and execution UI

#### Hooks (`src/hooks/`)
- **`useSystemAudio.ts`**: Manages system audio capture, VAD, transcription flow
- **`useCompletion.ts`**: Handles chat streaming, conversation management, file attachments
- **`useActionAssistant.ts`**: Manages action planning/execution workflow
- **`useApp.ts`**: Main app state (window visibility, conversation selection)
- **`useSettings.ts`**: Settings management (providers, models, system prompts)
- **`useHistory.ts`**: Conversation history CRUD operations
- **`useGlobalShortcuts.ts`**: Global keyboard shortcut management
- **`useWindow.ts`**: Window management (resize, hide/show)

#### Contexts (`src/contexts/`)
- **`AppProvider.tsx`**: Global state (providers, settings, license, trial status)
- **`ThemeProvider.tsx`**: Theme management (light/dark mode)

#### Libraries (`src/lib/`)
- **`functions.ts`**: API calling utilities (fetchAIResponse, fetchSTT)
- **`database.ts`**: SQLite operations (conversations, messages, system prompts)
- **`utils.ts`**: Helper functions (ID generation, debouncing, local storage)

---

### Tauri Backend Modules (`src-tauri/src/`)

#### Core Modules

**`lib.rs`** - Application Entry Point
- Initializes Tauri plugins (SQL, HTTP, keychain, shortcuts, etc.)
- Sets up global state (AudioState, CaptureState)
- Registers all Tauri commands
- Configures macOS-specific features (NSPanel)

**`api.rs`** - HTTP API Communication
- `transcribe_audio`: Sends audio to API server for transcription
- `chat_stream`: Handles SSE streaming from chat endpoint
- `fetch_models`: Retrieves available AI models
- `create_system_prompt`: Generates system prompts via API
- `submit_leave_application`: Submits leave applications
- `get_stored_credentials`: Retrieves license key, instance ID from secure storage

**`activate.rs`** - License Management
- `activate_license_api`: Activates license with server
- `validate_license_api`: Validates license status
- `deactivate_license_api`: Deactivates license
- `create_trial_license`: Creates trial license
- `secure_storage_save/get/remove`: Secure storage operations

#### Audio Capture Module (`speaker/`)

**`mod.rs`**: Platform-agnostic audio capture interface
- `SpeakerInput`: Trait for platform-specific audio capture
- Platform detection and routing

**`commands.rs`**: Main audio capture logic
- `start_system_audio_capture`: Starts audio capture with VAD
- `stop_system_audio_capture`: Stops audio capture
- `run_vad_capture`: VAD-enabled capture loop
  - Voice Activity Detection (RMS + peak threshold)
  - Noise gating
  - Pre-speech buffering
  - Speech start/end detection
  - Audio normalization
- `run_continuous_capture`: Manual recording mode
- `get_vad_config/update_vad_config`: VAD configuration management

**Platform-Specific Implementations:**
- **`macos.rs`**: CoreAudio via `cidre` crate
- **`windows.rs`**: WASAPI loopback capture
- **`linux.rs`**: PulseAudio monitor source capture

**VAD Configuration:**
```rust
pub struct VadConfig {
    enabled: bool,
    hop_size: usize,                      // Processing chunk size (1024)
    sensitivity_rms: f32,                 // RMS threshold (0.012)
    peak_threshold: f32,                  // Peak threshold (0.035)
    silence_chunks: usize,                // Silence duration to end (18 chunks)
    min_speech_chunks: usize,             // Minimum speech length (7 chunks)
    pre_speech_chunks: usize,             // Pre-speech buffer (12 chunks)
    noise_gate_threshold: f32,            // Noise gate (0.003)
    max_recording_duration_secs: u64,     // Max recording (180s)
}
```

#### Screen Capture Module (`capture.rs`)
- `start_screen_capture`: Captures full screen, creates overlay window
- `capture_selected_area`: Crops stored image to selected area
- `capture_to_base64`: Converts image to PNG base64
- Uses `xcap` for cross-platform screen capture
- Overlay window for selection UI (transparent, fullscreen, always-on-top)

#### Assistant Module (`assistant/`)

**`types.rs`**: Action system type definitions
- `ActionPlan`: List of actions to execute
- `Action`: Individual action (CreateFile, DeleteFile, MoveFile, etc.)
- `VerifiedPlan`: Validated action plan
- `ActionResult`: Execution result with undo information

**`commands.rs`**: Main action commands
- `parse_intent`: Deterministic intent parsing
- `plan_with_llm`: Validates LLM-generated action plan
- `verify_action_plan`: Verifies action plan safety
- `preview_action_plan`: Preview with risk score
- `execute_action_plan`: Executes verified plan
- `undo_action`: Undoes a previous action

**`planner/`**: Intent parsing and planning
- `deterministic.rs`: Rule-based intent parsing
- `verifier.rs`: Action plan verification (path validation, risk assessment)

**`executor/`**: Action execution
- `worker.rs`: Executes actions with undo support
- Sandboxed execution environment

**`policy.rs`**: Capability-based security
- `mint_capability_token`: Creates capability tokens with scopes

**`audit.rs`**: Audit logging
- `AuditLog`: Action audit trail
- `AuditEntry`: Individual audit entries

#### Database Module (`db/`)
- SQLite migrations for:
  - `system_prompts`: Custom system prompts
  - `conversations`: Chat conversation metadata
  - `messages`: Chat messages
  - `action_snapshots`: Action execution history
  - `audit_logs`: Action audit trail

#### Window & Shortcuts (`window.rs`, `shortcuts.rs`)
- Window management (height, visibility, positioning)
- Global keyboard shortcuts registration/handling
- macOS-specific window behaviors (NSPanel)

---

### API Server Modules (`scribe-api/src/`)

#### Routes (`routes/`)

**`chat.rs`** - Chat Endpoint
- `POST /api/v1/chat?stream=true`
- Accepts: user_message, system_prompt, image_base64, history
- Returns: SSE stream of AI response
- Headers: Authorization, license_key, instance, provider, model, machine_id

**`audio.rs`** - Transcription Endpoint
- `POST /api/v1/audio`
- Accepts: audio_base64 (WAV format)
- Returns: transcription text
- Uses WhisperService for STT

**`auth.rs`** - License Management
- `POST /api/v1/activate`: Activate license
- `POST /api/v1/deactivate`: Deactivate license
- `POST /api/v1/validate`: Validate license
- `GET /api/v1/checkout`: Get checkout URL
- `POST /api/v1/create-trial`: Create trial license

**`models.rs`** - Model Management
- `POST /api/v1/models`: List available models
- `POST /api/v1/prompt`: Generate system prompt

**`leave.rs`** - Leave Applications
- `POST /api/v1/leave-applications`: Submit leave application

**`health.rs`** - Health Checks
- `GET /health`: Health check
- `GET /api/v1/status`: API status

#### Services (`services/`)

**`openrouter.rs`** - OpenRouter Service
- Handles LLM API calls via OpenRouter
- Automatic model selection (text vs vision)
- SSE streaming support
- Multiple model format handling

**`whisper.rs`** - Whisper Service
- OpenAI Whisper API integration
- Base64 audio decoding
- WAV format handling

**`license.rs`** - License Service
- License validation logic
- Instance management
- Usage tracking
- Trial expiration handling

**`leave.rs`** - Leave Service
- Leave application processing
- Database persistence

#### Middleware (`middleware/`)

**`auth.rs`** - License Validation Middleware
- Validates license_key, instance_id, machine_id
- Checks license status (active, expired, suspended)
- Enforces instance limits

---

## 🔄 Data Flow Diagrams

### Audio Capture & Transcription Flow

```
┌──────────────┐
│ User Action  │
│ Start Capture│
└──────┬───────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Frontend: useSystemAudio hook           │
│ - Calls start_system_audio_capture      │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Tauri: speaker/commands.rs              │
│ - Platform-specific audio capture       │
│   (macOS: cidre, Windows: WASAPI,       │
│    Linux: PulseAudio)                   │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ VAD Processing Loop                     │
│ - Noise gate filtering                  │
│ - RMS + peak detection                  │
│ - Speech start/end detection            │
│ - Pre-speech buffering                  │
│ - Audio normalization                   │
└──────┬──────────────────────────────────┘
       │ (speech detected)
       ▼
┌─────────────────────────────────────────┐
│ Tauri: Emit "speech-detected" event     │
│ - Base64 encoded WAV audio              │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Frontend: Receive event                 │
│ - Convert base64 to Blob                │
│ - Call transcribe_audio command         │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Tauri: api.rs::transcribe_audio         │
│ - POST /api/v1/audio                    │
│ - Headers: license_key, instance,       │
│            machine_id, API_ACCESS_KEY   │
│ - Body: { audio_base64 }                │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ API Server: routes/audio.rs             │
│ - Validates license                     │
│ - Calls WhisperService                  │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ WhisperService: OpenAI Whisper API      │
│ - Transcribes audio                     │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Frontend: Receive transcription         │
│ - Display in UI                         │
│ - Auto-submit to chat (if enabled)      │
└─────────────────────────────────────────┘
```

### Chat Flow with Streaming

```
┌──────────────┐
│ User Input   │
│ (text/image) │
└──────┬───────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Frontend: useCompletion hook            │
│ - Calls chat_stream command             │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Tauri: api.rs::chat_stream              │
│ - Gets stored credentials               │
│ - Prepares ChatRequest                  │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ POST /api/v1/chat?stream=true           │
│ Headers:                                │
│   - Authorization: Bearer {API_KEY}     │
│   - license_key, instance, provider,    │
│     model, machine_id                   │
│ Body:                                   │
│   - user_message                        │
│   - system_prompt (optional)            │
│   - image_base64 (optional)             │
│   - history (optional)                  │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ API Server: routes/chat.rs              │
│ - Validates license                     │
│ - Calls OpenRouterService               │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ OpenRouterService                       │
│ - Selects model (text vs vision)        │
│ - Calls OpenRouter API                  │
│ - Returns SSE stream                    │
└──────┬──────────────────────────────────┘
       │ (SSE stream)
       ▼
┌─────────────────────────────────────────┐
│ Tauri: Process SSE stream               │
│ - Parse JSON chunks                     │
│ - Extract text content                  │
│ - Emit "chat_stream_chunk" events       │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Frontend: Listen to events              │
│ - Update UI with streaming text         │
│ - Display in real-time                  │
└──────┬──────────────────────────────────┘
       │ (stream complete)
       ▼
┌─────────────────────────────────────────┐
│ Tauri: Emit "chat_stream_complete"      │
│ - Full response text                    │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Frontend: Save to database              │
│ - Save conversation                     │
│ - Save messages                         │
└─────────────────────────────────────────┘
```

### Screen Capture Flow

```
┌──────────────┐
│ User Action  │
│ Screenshot   │
│ Shortcut     │
└──────┬───────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Frontend: Call start_screen_capture     │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Tauri: capture.rs::start_screen_capture │
│ - Capture full screen (xcap)            │
│ - Store in CaptureState                 │
│ - Create overlay window                 │
│   (transparent, fullscreen, on-top)     │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Frontend: Overlay.tsx                   │
│ - Render selection UI                   │
│ - User selects area                     │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Frontend: Call capture_selected_area    │
│ - Pass selection coordinates            │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Tauri: capture.rs::capture_selected_area│
│ - Crop stored image                     │
│ - Encode to PNG base64                  │
│ - Emit "captured-selection" event       │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Frontend: Receive base64 image          │
│ - Display preview                       │
│ - Attach to chat (if needed)            │
│ - Send to AI for analysis               │
└─────────────────────────────────────────┘
```

### License Activation Flow

```
┌──────────────┐
│ User Enters  │
│ License Key  │
└──────┬───────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Frontend: Call activate_license_api     │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Tauri: activate.rs::activate_license_api│
│ - Get machine_id                        │
│ - Generate instance_id (UUID)           │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ POST /api/v1/activate                   │
│ Body: { license_key, machine_id }       │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ API Server: routes/auth.rs              │
│ - Validate license_key exists           │
│ - Check license is active               │
│ - Create license_instance record        │
│ - Check instance limits                 │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Tauri: Store credentials                │
│ - license_key                           │
│ - instance_id                           │
│ - selected_model (optional)             │
│ - Secure storage (OS keychain)          │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Frontend: Update license status         │
│ - Show activation success               │
│ - Enable features                       │
└─────────────────────────────────────────┘
```

---

## 🔌 API Reference

### Tauri Commands

#### Audio Capture
- `start_system_audio_capture(vad_config?, device_id?)`: Start audio capture
- `stop_system_audio_capture()`: Stop audio capture
- `manual_stop_continuous()`: Stop continuous recording
- `get_vad_config()`: Get VAD configuration
- `update_vad_config(config)`: Update VAD configuration
- `check_system_audio_access()`: Check audio permissions
- `request_system_audio_access()`: Request audio permissions

#### API Communication
- `transcribe_audio(audio_base64)`: Transcribe audio
- `chat_stream(user_message, system_prompt?, image_base64?, history?)`: Stream chat response
- `fetch_models()`: Fetch available models
- `create_system_prompt(user_prompt)`: Generate system prompt
- `submit_leave_application(payload)`: Submit leave application

#### License Management
- `activate_license_api(license_key)`: Activate license
- `validate_license_api()`: Validate license
- `deactivate_license_api()`: Deactivate license
- `create_trial_license()`: Create trial license
- `secure_storage_save(key, value)`: Save to secure storage
- `secure_storage_get(key)`: Get from secure storage

#### Screen Capture
- `start_screen_capture()`: Start screen capture
- `capture_selected_area(coords)`: Capture selected area
- `capture_to_base64()`: Capture full screen to base64
- `close_overlay_window()`: Close capture overlay

#### Assistant Actions
- `parse_intent(user_input)`: Parse user intent
- `plan_with_llm(plan)`: Validate LLM plan
- `verify_action_plan(plan)`: Verify action plan
- `execute_action_plan(plan, confirm_token?)`: Execute action plan
- `undo_action(action_id)`: Undo action

#### Window & Shortcuts
- `set_window_height(height)`: Set window height
- `check_shortcuts_registered()`: Check shortcuts
- `update_shortcuts(shortcuts)`: Update shortcuts
- `set_always_on_top(enabled)`: Set always on top

### API Server Endpoints

#### Authentication Headers
All endpoints (except `/health`, `/api/v1/status`, `/api/v1/create-trial`) require:
```
Authorization: Bearer {API_ACCESS_KEY}
license_key: {license_key}
instance: {instance_id}
machine_id: {machine_id}
```

#### License Management
- `POST /api/v1/activate`
  - Body: `{ license_key, machine_id }`
  - Returns: `{ instance_id, license_status }`

- `POST /api/v1/validate`
  - Body: `{ license_key, instance_id, machine_id }`
  - Returns: `{ valid: bool, status: string }`

- `POST /api/v1/deactivate`
  - Body: `{ license_key, instance_id, machine_id }`

- `POST /api/v1/create-trial`
  - Body: `{ machine_id }`
  - Returns: `{ license_key, trial_ends_at }`

#### AI Services
- `POST /api/v1/chat?stream=true`
  - Headers: `provider`, `model`
  - Body: `{ user_message, system_prompt?, image_base64?, history? }`
  - Returns: SSE stream

- `POST /api/v1/audio`
  - Body: `{ audio_base64 }`
  - Returns: `{ transcription: string }`

- `POST /api/v1/models`
  - Returns: `{ models: Model[] }`

- `POST /api/v1/prompt`
  - Body: `{ user_prompt }`
  - Returns: `{ prompt_name, system_prompt }`

#### Health
- `GET /health`: Health check
- `GET /api/v1/status`: API status

---

## 🗄 Database Schema

### SQLite (Desktop App)

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

### PostgreSQL (API Server)

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

#### leave_applications
```sql
CREATE TABLE leave_applications (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    usn TEXT NOT NULL,
    department TEXT NOT NULL,
    reason TEXT NOT NULL,
    summary TEXT NOT NULL,
    attachments JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

---

## 🔒 Security & Authentication

### License System
- **License Key**: Stored in OS secure storage (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- **Instance ID**: UUID generated per machine on activation
- **Machine ID**: Hardware-based identifier (CPU, motherboard, etc.)
- **Validation**: Periodic checks against API server
- **Trial System**: Time-limited trial licenses with expiration tracking

### API Authentication
- **API_ACCESS_KEY**: Bearer token for API server authentication
- **License Key**: Header-based validation
- **Machine ID**: Device fingerprinting for multi-device support
- **Instance ID**: Tracks individual device activations

### Secure Storage
- Uses OS-native secure storage:
  - macOS: Keychain Services
  - Windows: Credential Manager
  - Linux: Secret Service API (via libsecret)
- Stores: license_key, instance_id, selected_model

### Action System Security
- **Capability Tokens**: Scope-based permissions
- **Plan Verification**: Path validation, risk assessment
- **Audit Logging**: Complete action history
- **Undo Support**: Reversible actions with backup

---

## 🖥 Platform-Specific Implementation

### macOS
- **Audio Capture**: CoreAudio via `cidre` crate
- **Window Management**: NSPanel (floating panel window)
- **Permissions**: macOS permissions plugin for audio access
- **Auto-start**: LaunchAgent

### Windows
- **Audio Capture**: WASAPI loopback
- **Window Management**: Standard Windows window behavior
- **Auto-start**: Task Scheduler

### Linux
- **Audio Capture**: PulseAudio monitor source
- **Window Management**: Standard X11/Wayland behavior
- **Desktop Integration**: `.desktop` file support
- **Auto-start**: Systemd user service or `.desktop` autostart

---

## 📝 Configuration & Environment

### Frontend Environment Variables
- None required (all config via API/Tauri commands)

### Tauri Environment Variables
- `APP_ENDPOINT`: API server URL
- `API_ACCESS_KEY`: API authentication key
- `PAYMENT_ENDPOINT`: Payment/license endpoint
- `POSTHOG_API_KEY`: Analytics (optional)

### API Server Environment Variables
- `DATABASE_URL`: PostgreSQL connection string
- `LEAVE_DATABASE_URL`: Leave applications database URL
- `OPENAI_API_KEY`: Whisper API key
- `OPENROUTER_API_KEY`: OpenRouter API key
- `API_ACCESS_KEY`: Server authentication key
- `PORT`: Server port (default: 8080)

---

## 🚀 Build & Deployment

### Frontend Build
```bash
npm install
npm run build          # Build frontend
npm run tauri build    # Build Tauri app
```

**Output**: `src-tauri/target/release/bundle/`
- `deb/`: Debian packages
- `rpm/`: RPM packages
- `appimage/`: AppImage files
- `macos/`: .dmg files
- `msi/` or `nsis/`: Windows installers

### API Server Build
```bash
cd scribe-api
cargo build --release
```

**Deployment**: Systemd service or Docker container

---

## 🎨 Key Features Deep Dive

### Voice Activity Detection (VAD)
- **Noise Gating**: Filters background noise before VAD
- **RMS Detection**: Root Mean Square energy level detection
- **Peak Detection**: Peak amplitude threshold
- **Pre-speech Buffering**: Captures speech onset naturally
- **Silence Detection**: Configurable silence duration to end recording
- **Audio Normalization**: Normalizes audio levels for consistent transcription

### Action Execution System
- **Intent Parsing**: Natural language to structured actions
- **LLM Planning**: AI-generated action plans with verification
- **Plan Verification**: Safety checks (path validation, risk assessment)
- **Sandboxed Execution**: Isolated execution environment
- **Audit Logging**: Complete action history
- **Undo Support**: Reversible actions

### Streaming Chat
- **Server-Sent Events (SSE)**: Real-time text streaming
- **Multiple Format Support**: Handles various LLM response formats
- **Chunk Processing**: Efficient chunk-by-chunk processing
- **Error Handling**: Graceful error recovery

---

This architecture document provides a comprehensive overview of the Ghost (Scribe) system. For implementation details, refer to the source code in each module.

