# Ghost (Scribe) - Complete System Architecture Documentation

## Table of Contents
1. [System Overview](#system-overview)
2. [Technology Stack](#technology-stack)
3. [Architecture Layers](#architecture-layers)
4. [Core Components](#core-components)
5. [Data Flow](#data-flow)
6. [Database Schema](#database-schema)
7. [Security & Authentication](#security--authentication)
8. [Platform-Specific Features](#platform-specific-features)
9. [Key Features Deep Dive](#key-features-deep-dive)
10. [File Structure](#file-structure)

---

## System Overview

**Ghost (Scribe)** is a privacy-first, cross-platform AI desktop assistant built with Tauri (Rust backend + React/TypeScript frontend). It provides:

- **Real-time System Audio Capture**: Captures speaker output during meetings, calls, and conversations
- **Speech-to-Text Transcription**: Converts audio to text using configurable STT providers
- **AI-Powered Chat**: Multi-provider AI chat with streaming responses
- **Screen Capture & Analysis**: Screenshot capture with AI-powered image analysis
- **Action Execution System**: Natural language to file system operations
- **License Management**: Trial system and license activation
- **Cross-Platform Support**: macOS, Windows, and Linux

The system consists of three main parts:
1. **Desktop Application** (Tauri + React)
2. **API Server** (Rust/Axum)
3. **Database Layer** (SQLite for desktop, PostgreSQL for API)

---

## Technology Stack

### Frontend
- **Framework**: React 19.1.0 with TypeScript
- **Build Tool**: Vite 7.0.4
- **UI Library**: 
  - Radix UI (headless components)
  - Tailwind CSS 4.1.12 (styling)
- **State Management**: React Context API
- **Audio Processing**: `@ricky0123/vad-react` (Voice Activity Detection)
- **Markdown Rendering**: `react-markdown` with Shiki for syntax highlighting
- **Icons**: Lucide React

### Backend (Tauri)
- **Framework**: Tauri 2.5.2
- **Language**: Rust (Edition 2021)
- **Async Runtime**: Tokio
- **Audio Capture**:
  - macOS: `cidre` (CoreAudio wrapper)
  - Windows: `wasapi`
  - Linux: `libpulse-binding` (PulseAudio)
- **Screen Capture**: `xcap`
- **Database**: SQLite via `tauri-plugin-sql`
- **HTTP Client**: `reqwest` with streaming support
- **Image Processing**: `image` crate

### API Server (scribe-api)
- **Framework**: Axum (Rust web framework)
- **Database**: PostgreSQL via SQLx
- **HTTP Client**: Reqwest
- **Streaming**: Server-Sent Events (SSE) for chat responses
- **Services**: 
  - OpenRouter (LLM API gateway)
  - OpenAI Whisper (Speech-to-text)
- **Authentication**: Bearer token + license key validation

---

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│                    Frontend Layer (React)                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │  Components  │  │     Hooks    │  │   Contexts   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   UI State   │  │   Event      │  │   Storage     │     │
│  │  Management  │  │   Listeners  │  │   (localStorage)│   │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└───────────────────────┬─────────────────────────────────────┘
                        │ Tauri Commands (invoke)
                        │ Tauri Events (listen/emit)
┌───────────────────────▼─────────────────────────────────────┐
│              Tauri Backend Layer (Rust)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Commands   │  │    Events     │  │   Platform    │     │
│  │   (API)      │  │   (Emitter)   │  │     APIs      │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Audio      │  │   Screen      │  │   Database   │     │
│  │   Capture    │  │   Capture     │  │   (SQLite)   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │  Assistant   │  │  Shortcuts   │  │   Window     │     │
│  │   System     │  │   Manager     │  │   Manager    │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└───────────────────────┬─────────────────────────────────────┘
                        │ HTTP Requests (reqwest)
                        │ Secure Storage (keychain)
┌───────────────────────▼─────────────────────────────────────┐
│            Scribe API Server (Axum/Rust)                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │    Routes    │  │   Services   │  │  Middleware  │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   License    │  │  OpenRouter  │  │   Whisper    │     │
│  │   Service    │  │   Service    │  │   Service    │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐                        │
│  │  PostgreSQL  │  │   Leave      │                        │
│  │   Database   │  │   Service    │                        │
│  └──────────────┘  └──────────────┘                        │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Components

### Frontend Components

#### Main Application (`src/App.tsx`)
- Root component that orchestrates the entire UI
- Manages window visibility and layout
- Renders different views based on state (capturing, chat, settings)
- Handles trial expiration warnings

#### Completion System (`src/components/completion/`)
- **Input.tsx**: Text input with file attachments
- **index.tsx**: Main completion UI with streaming response display
- **Files.tsx**: File attachment management
- **Screenshot.tsx**: Screenshot capture integration
- **Audio.tsx**: Audio file handling
- **AutoSpeechVad.tsx**: Automatic speech-to-text integration

#### Speech System (`src/components/speech/`)
- **index.tsx**: Main speech interface
- **Header.tsx**: Speech capture controls
- **OperationSection.tsx**: Transcription and AI response display
- **Context.tsx**: Context management for conversations
- **QuickActions.tsx**: Pre-defined action buttons
- **VadConfigPanel.tsx**: Voice Activity Detection configuration
- **AudioVisualizer.tsx**: Real-time audio waveform visualization
- **StatusIndicator.tsx**: Visual status indicators
- **PermissionFlow.tsx**: Permission request UI
- **SetupInstructions.tsx**: Setup guide

#### Settings (`src/components/settings/`)
- **index.tsx**: Main settings panel
- **ai-configs/**: AI provider configuration
- **stt-configs/**: Speech-to-text provider configuration
- **system-prompt/**: System prompt management
- **shortcuts/**: Keyboard shortcut configuration
- **Theme.tsx**: Theme selection
- **AudioSelection.tsx**: Audio device selection
- **ScreenshotConfigs.tsx**: Screenshot behavior configuration
- **ScribeApiSetup.tsx**: API server configuration

#### History (`src/components/history/`)
- **ChatHistory.tsx**: Conversation list sidebar
- **MessageHistory.tsx**: Message history view
- **ConversationListView.tsx**: List of all conversations
- **ConversationItem.tsx**: Individual conversation item
- **DeleteConfirmationDialog.tsx**: Confirmation for deletion

#### Assistant (`src/components/assistant/`)
- **ActionMode.tsx**: Action execution interface
- **ActionPreview.tsx**: Preview of planned actions
- **ActionHistory.tsx**: History of executed actions
- **PathPromptDialog.tsx**: Path input dialog

#### Overlay (`src/components/Overlay.tsx`)
- Full-screen overlay for screen capture selection
- Renders on separate window (`capture-overlay`)
- Handles mouse selection and coordinate capture

### React Hooks

#### `useSystemAudio` (`src/hooks/useSystemAudio.ts`)
- Manages system audio capture state
- Handles VAD (Voice Activity Detection) configuration
- Processes speech detection events
- Manages transcription and AI response flow
- Supports both VAD mode and continuous recording mode
- Handles conversation management for system audio

#### `useCompletion` (`src/hooks/useCompletion.ts`)
- Manages chat completion state
- Handles streaming AI responses
- Manages file attachments (images, audio)
- Handles screenshot capture and submission
- Manages conversation history
- Supports leave application submission flow

#### `useActionAssistant` (`src/hooks/useActionAssistant.ts`)
- Manages action planning and execution
- Parses user intent into action plans
- Previews actions before execution
- Executes actions with rollback support
- Manages action history and audit logs

#### `useApp` (`src/contexts/app.context.tsx`)
- Global application state management
- Provider/STT configuration
- System prompt management
- License status
- Customizable settings (cursor, window behavior, etc.)

### Tauri Backend Modules

#### Core Modules

**`lib.rs`** - Application Entry Point
- Initializes Tauri application
- Sets up plugins (SQL, HTTP, keychain, autostart, etc.)
- Configures global shortcuts
- Handles macOS-specific panel setup
- Manages environment variable loading

**`api.rs`** - HTTP API Communication
- `transcribe_audio`: Sends audio to API server for transcription
- `chat_stream`: Streams chat responses from API server
- `fetch_models`: Fetches available AI models
- `create_system_prompt`: Generates system prompts via API
- `submit_leave_application`: Submits leave applications
- Secure credential storage management

**`activate.rs`** - License Management
- `activate_license_api`: Activates license with API server
- `validate_license_api`: Validates current license
- `create_trial_license`: Creates trial license
- `get_checkout_url`: Gets payment URL
- Secure storage operations (keychain)

**`speaker/`** - Audio Capture Module
- **`mod.rs`**: Module interface
- **`commands.rs`**: Tauri commands for audio capture
  - `start_system_audio_capture`: Starts audio capture
  - `stop_system_audio_capture`: Stops audio capture
  - `get_vad_config`: Gets VAD configuration
  - `update_vad_config`: Updates VAD configuration
- **`macos.rs`**: macOS CoreAudio implementation
- **`windows.rs`**: Windows WASAPI implementation
- **`linux.rs`**: Linux PulseAudio implementation

**`capture.rs`** - Screen Capture
- `start_screen_capture`: Captures full screen and opens overlay
- `capture_selected_area`: Crops captured image to selection
- `capture_to_base64`: Captures and encodes to base64
- `close_overlay_window`: Closes overlay window

**`shortcuts.rs`** - Global Keyboard Shortcuts
- Registers and manages global shortcuts
- Handles shortcut actions (show/hide, screenshot, audio)
- Validates shortcut combinations
- Manages window visibility state

**`window.rs`** - Window Management
- `set_window_height`: Dynamically adjusts window height
- Window positioning and behavior

**`assistant/`** - Action System
- **`commands.rs`**: Tauri commands for actions
  - `parse_intent`: Parses user input into actions
  - `plan_with_llm`: Uses LLM to create action plan
  - `verify_action_plan`: Validates action plan
  - `preview_action_plan`: Previews actions before execution
  - `execute_action_plan`: Executes actions
  - `undo_action`: Reverts actions
  - `get_audit_history`: Gets action history
  - `mint_capability_token`: Creates permission tokens
- **`types.rs`**: Action schema definitions
- **`planner/`**: Action planning logic
  - `deterministic.rs`: Rule-based intent parsing
  - `verifier.rs`: Plan verification
- **`executor/`**: Action execution
  - `worker.rs`: Main execution worker
  - `fs_adapter.rs`: File system operations
  - `snapshot.rs`: Snapshot management for undo
- **`validator.rs`**: Action validation
- **`policy.rs`**: Capability-based permissions
- **`audit.rs`**: Audit logging
- **`sandbox.rs`**: Sandboxed execution (future)

**`db/`** - Database Migrations
- SQLite migrations for:
  - `system_prompts`: User-defined system prompts
  - `conversations`: Chat conversations
  - `messages`: Individual messages
  - `action_snapshots`: Action execution snapshots
  - `audit_logs`: Action audit trail

### API Server Components

**`main.rs`** - Server Entry Point
- Initializes Axum router
- Sets up CORS
- Configures middleware
- Runs database migrations
- Starts HTTP server

**`routes/`** - API Endpoints
- `health.rs`: Health check endpoint
- `auth.rs`: License activation/validation
- `chat.rs`: Streaming chat endpoint
- `audio.rs`: Audio transcription endpoint
- `models.rs`: Model listing and prompt generation
- `leave.rs`: Leave application submission
- `updates.rs`: Tauri updater manifest

**`services/`** - Business Logic
- `license.rs`: License management
- `openrouter.rs`: OpenRouter API integration
- `whisper.rs`: OpenAI Whisper integration
- `leave.rs`: Leave application handling

**`models/`** - Data Models
- Database models for licenses, instances, usage logs

**`middleware/`** - Request Middleware
- `auth.rs`: License validation middleware
- Request logging

**`db/`** - Database Setup
- Connection pool management
- Migration runner

---

## Data Flow

### 1. Audio Capture & Transcription Flow

```
User Action: Start System Audio Capture
    ↓
Frontend: useSystemAudio.startCapture()
    ↓
Tauri: start_system_audio_capture command
    ↓
Platform Audio Capture (macOS/Windows/Linux)
    ├─ macOS: CoreAudio via cidre
    ├─ Windows: WASAPI
    └─ Linux: PulseAudio
    ↓
Audio Stream Processing
    ├─ VAD (Voice Activity Detection)
    │   ├─ Noise Gate Filter
    │   ├─ RMS Level Detection
    │   ├─ Peak Detection
    │   └─ Speech Start/End Detection
    └─ Audio Chunking (hop_size: 1024 samples)
    ↓
Speech Detected Event
    ├─ Tauri emits "speech-detected" event
    └─ Payload: base64-encoded WAV audio
    ↓
Frontend: Receives event via listen("speech-detected")
    ↓
Frontend: Converts base64 to Blob
    ↓
Frontend: Calls transcribe_audio Tauri command
    ↓
Tauri: api::transcribe_audio
    ├─ Gets stored credentials (license_key, instance_id)
    ├─ Gets machine_id
    └─ POST to /api/v1/audio
        Headers:
        - Authorization: Bearer {API_ACCESS_KEY}
        - license_key: {license_key}
        - instance: {instance_id}
        - machine_id: {machine_id}
        Body: { audio_base64: string }
    ↓
API Server: routes::audio::transcribe
    ├─ Validates license via middleware
    ├─ Calls WhisperService
    └─ Returns { success: true, transcription: string }
    ↓
Frontend: Receives transcription
    ↓
Frontend: If auto-submit enabled → processWithAI()
    └─ Otherwise: Display transcription for user review
```

### 2. Chat Flow (Streaming)

```
User Input: Text message or transcribed audio
    ↓
Frontend: useCompletion.submit()
    ↓
Tauri: api::chat_stream command
    ├─ Gets stored credentials
    ├─ Gets selected model (provider, model_id)
    ├─ Prepares ChatRequest:
    │   {
    │     user_message: string,
    │     system_prompt?: string,
    │     image_base64?: string[],
    │     history?: string (JSON)
    │   }
    └─ POST to /api/v1/chat?stream=true
        Headers:
        - Authorization: Bearer {API_ACCESS_KEY}
        - license_key: {license_key}
        - instance: {instance_id}
        - provider: {provider}
        - model: {model_id}
        - machine_id: {machine_id}
    ↓
API Server: routes::chat::chat
    ├─ Validates license
    ├─ Calls OpenRouterService
    └─ Returns SSE stream
    ↓
Tauri: Processes SSE stream
    ├─ Parses "data: {json}" chunks
    ├─ Extracts text content (multiple format support)
    │   - OpenAI format: choices[0].delta.content
    │   - Anthropic format: delta.text
    │   - Google format: candidates[0].content.parts[0].text
    │   - Recursive search fallback
    └─ Emits "chat_stream_chunk" events
    ↓
Frontend: Listens to "chat_stream_chunk" events
    ↓
Frontend: Updates UI with streaming text
    ↓
Tauri: On stream completion
    └─ Emits "chat_stream_complete" with full response
    ↓
Frontend: Saves conversation to SQLite
    ├─ Creates/updates conversation record
    ├─ Creates message records (user + assistant)
    └─ Updates conversation title (if new)
```

### 3. Screen Capture Flow

```
User Action: Trigger Screenshot Shortcut
    ↓
Frontend: useCompletion.captureScreenshot()
    ↓
Tauri: start_screen_capture command
    ├─ Captures full screen using xcap
    ├─ Stores image in CaptureState
    └─ Creates overlay window
        - Fullscreen, transparent
        - Always on top
        - No decorations
        - Label: "capture-overlay"
    ↓
Frontend: Overlay.tsx renders (detected by window label)
    ├─ Shows selection UI
    └─ Handles mouse drag for selection
    ↓
User: Selects area with mouse
    ↓
Frontend: Overlay calls capture_selected_area with coordinates
    ↓
Tauri: capture::capture_selected_area
    ├─ Crops stored image to selection
    ├─ Encodes to PNG base64
    └─ Emits "captured-selection" event
    ↓
Frontend: Receives "captured-selection" event
    ↓
Frontend: Based on screenshot configuration
    ├─ Auto mode: Submit directly to AI with prompt
    └─ Manual mode: Add to attached files
```

### 4. License Activation Flow

```
User: Enters License Key
    ↓
Frontend: Calls activate_license_api
    ↓
Tauri: activate::activate_license_api
    ├─ Gets machine_id (hardware-based)
    ├─ Generates instance_id (UUID)
    └─ POST to /api/v1/activate
        Body: {
          license_key: string,
          machine_id: string,
          instance_name: string (optional)
        }
    ↓
API Server: routes::auth::activate
    ├─ Validates license_key exists and is active
    ├─ Checks max_instances limit
    ├─ Creates license_instance record
    └─ Returns {
         activated: true,
         instance: { id, name, created_at }
       }
    ↓
Tauri: Stores credentials in secure storage
    ├─ license_key
    ├─ instance_id
    └─ selected_model (optional)
    ↓
Frontend: Updates license status UI
```

### 5. Action Execution Flow

```
User Input: "Create a file called test.txt with content 'Hello'"
    ↓
Frontend: useActionAssistant.parseIntent()
    ↓
Tauri: assistant::commands::parse_intent
    ├─ Uses deterministic parser (regex patterns)
    └─ Returns ActionPlan
        {
          id: "uuid",
          origin: { user_input, source: "ui", request_id },
          actions: [{
            id: "uuid",
            type: "fs_create_file",
            args: { path: "/absolute/path/test.txt", content: "Hello" },
            preconditions: { exists: false, writable: true }
          }],
          summary: "Create file test.txt",
          risk_score: 0.2,
          dry_run: true
        }
    ↓
Frontend: useActionAssistant.previewAction()
    ↓
Tauri: assistant::commands::preview_action_plan
    ├─ Validates plan structure
    ├─ Checks file system preconditions
    └─ Returns PreviewResult
        {
          plan: ActionPlan,
          warnings: [],
          can_execute: true
        }
    ↓
Frontend: Shows preview to user
    ↓
User: Confirms execution
    ↓
Frontend: useActionAssistant.executeAction()
    ↓
Tauri: assistant::commands::execute_action_plan
    ├─ Creates snapshot (for undo)
    ├─ Validates capability token (if provided)
    ├─ Executes each action via executor::worker
    │   ├─ fs_adapter performs file operations
    │   └─ Creates snapshot after each action
    └─ Returns ActionResult
        {
          success: true,
          executed_actions: [...],
          snapshot_id: "uuid"
        }
    ↓
Frontend: Updates UI with result
    ↓
Tauri: Saves to audit_logs table
```

---

## Database Schema

### SQLite (Desktop App)

#### `system_prompts`
```sql
CREATE TABLE system_prompts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    prompt TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### `conversations`
```sql
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    title TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### `messages`
```sql
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    role TEXT NOT NULL,  -- 'user' | 'assistant' | 'system'
    content TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id)
);
```

#### `action_snapshots`
```sql
CREATE TABLE action_snapshots (
    id TEXT PRIMARY KEY,
    action_plan TEXT NOT NULL,  -- JSON string
    executed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### `audit_logs`
```sql
CREATE TABLE audit_logs (
    id TEXT PRIMARY KEY,
    action_id TEXT NOT NULL,
    action_type TEXT NOT NULL,
    details TEXT,  -- JSON string
    executed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### PostgreSQL (API Server)

#### `licenses`
```sql
CREATE TABLE licenses (
    id UUID PRIMARY KEY,
    license_key TEXT UNIQUE NOT NULL,
    user_id UUID,
    status TEXT,  -- 'active' | 'suspended' | 'expired'
    tier TEXT,  -- 'trial' | 'free' | 'basic' | 'pro' | 'enterprise'
    max_instances INTEGER,
    is_trial BOOLEAN,
    trial_ends_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### `license_instances`
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

#### `usage_logs`
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

#### `leave_applications`
```sql
CREATE TABLE leave_applications (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    usn TEXT NOT NULL,
    department TEXT NOT NULL,
    reason TEXT NOT NULL,
    summary TEXT NOT NULL,
    attachments JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

## Security & Authentication

### License System
- **License Key**: Stored in OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- **Instance ID**: UUID generated per machine installation
- **Machine ID**: Hardware-based identifier (CPU, motherboard, etc.)
- **Validation**: Periodic checks against API server
- **Trial System**: 14-day trial licenses with automatic creation on first launch

### API Authentication
- **API_ACCESS_KEY**: Bearer token for API server authentication
- **License Key**: Header-based validation on each request
- **Machine ID**: Device fingerprinting for security
- **Instance ID**: Multi-device support (one license, multiple machines)

### Secure Storage
- Uses OS-native keychain services
- Stores: `license_key`, `instance_id`, `selected_model`
- Path: `{app_data_dir}/secure_storage.json` (encrypted by OS)

### Capability Tokens (Action System)
- JWT-based tokens for action permissions
- Scoped to specific action types and paths
- Validated before action execution

---

## Platform-Specific Features

### macOS
- **NSPanel**: Floating panel window using `tauri-nspanel`
- **macOS Private API**: Enhanced window behavior
- **Audio Capture**: CoreAudio via `cidre` crate
- **Permissions**: macOS permissions plugin for screen recording
- **Window Level**: NSFloatWindowLevel for floating behavior

### Windows
- **WASAPI**: Audio capture via Windows Audio Session API
- **Window Management**: Special handling for Windows window behavior
- **Screen Capture**: xcap with Windows-specific optimizations

### Linux
- **PulseAudio**: Audio capture via `libpulse-binding`
- **Desktop Integration**: `.desktop` file support
- **Screen Capture**: xcap with X11/Wayland support

---

## Key Features Deep Dive

### 1. Voice Activity Detection (VAD)

**Configuration Parameters:**
- `enabled`: Enable/disable VAD (if disabled, uses continuous mode)
- `hop_size`: Audio chunk size (1024 samples default)
- `sensitivity_rms`: RMS threshold for speech detection (0.012 default)
- `peak_threshold`: Peak amplitude threshold (0.035 default)
- `silence_chunks`: Chunks of silence before stopping (18 = ~0.4s)
- `min_speech_chunks`: Minimum speech chunks to capture (7 = ~0.16s)
- `pre_speech_chunks`: Chunks to capture before speech start (12 = ~0.27s)
- `noise_gate_threshold`: Noise filtering threshold (0.003 default)
- `max_recording_duration_secs`: Maximum recording length (180s default)

**Flow:**
1. Audio stream is chunked into `hop_size` samples
2. Each chunk is analyzed for RMS level and peak amplitude
3. Noise gate filters out low-level noise
4. Speech start detected when RMS/peak exceeds thresholds
5. Pre-speech chunks are included (catches word beginnings)
6. Speech end detected after `silence_chunks` of silence
7. Minimum speech length enforced (`min_speech_chunks`)
8. Audio encoded to WAV base64 and emitted as event

### 2. Streaming Chat Responses

**SSE Format:**
```
data: {"choices":[{"delta":{"content":"Hello"}}]}

data: {"choices":[{"delta":{"content":" world"}}]}

data: [DONE]
```

**Processing:**
- Tauri receives SSE stream chunks
- Parses JSON from `data: ` prefix
- Extracts text using multiple format heuristics
- Emits incremental chunks to frontend
- Frontend appends chunks to display
- On `[DONE]`, emits completion event with full response

### 3. Action Planning System

**Action Schema v2:**
```json
{
  "id": "uuid",
  "origin": {
    "user_input": "create file test.txt",
    "source": "ui|voice|automation|plugin",
    "request_id": "uuid"
  },
  "actions": [{
    "id": "uuid",
    "type": "fs_create_file|fs_read_file|fs_copy_file|fs_move_file|fs_delete_file|fs_create_directory",
    "args": {
      "path": "/absolute/path",
      "content": "string (for create)",
      "source_path": "string (for copy/move)",
      "destination_path": "string (for copy/move)"
    },
    "preconditions": {
      "exists": true|false,
      "writable": true|false,
      "readable": true|false
    },
    "metadata": {
      "confidence": 0.0-1.0
    }
  }],
  "summary": "Brief description",
  "risk_score": 0.0-1.0,
  "dry_run": true
}
```

**Planning Methods:**
1. **Deterministic Parser**: Regex-based intent parsing
   - Patterns for common file operations
   - Fast, no API calls
   - Limited to predefined patterns

2. **LLM Planner**: Uses AI to generate action plans
   - More flexible, understands natural language
   - Requires API call
   - Validated after generation

**Execution Safety:**
- Preconditions checked before execution
- Snapshots created for undo support
- Capability tokens for permission control
- Audit logging for all actions
- Transactional execution (all or nothing)

### 4. Custom Provider System

**AI Providers:**
- Built-in providers (OpenRouter, OpenAI, Anthropic, etc.)
- Custom providers via cURL command configuration
- Variables substitution (API keys, etc.)
- Provider selection stored in localStorage

**STT Providers:**
- Built-in providers (Whisper, Deepgram, etc.)
- Custom providers via cURL command
- `AUDIO_BASE64` placeholder for audio data
- Provider selection stored in localStorage

**Configuration Format:**
```json
{
  "id": "custom-provider-1",
  "name": "My Custom Provider",
  "curl": "curl -X POST https://api.example.com/chat \\\n  -H 'Authorization: Bearer {API_KEY}' \\\n  -H 'Content-Type: application/json' \\\n  -d '{\"message\": \"{MESSAGE}\"}'",
  "isCustom": true,
  "variables": {
    "API_KEY": "your-api-key"
  }
}
```

### 5. Conversation Management

**Conversation Lifecycle:**
1. **Creation**: New conversation ID generated on first message
2. **Title Generation**: AI-generated title from first user message
3. **Message Storage**: Messages stored with conversation_id foreign key
4. **History Loading**: Conversations loaded from SQLite on app start
5. **Debounced Saving**: Saves 500ms after last change to prevent race conditions

**Conversation Types:**
- **Chat Conversations**: Regular text-based conversations
- **System Audio Conversations**: Conversations from audio transcription
- **Screenshot Conversations**: Conversations with screenshot analysis

---

## File Structure

```
scribe/
├── src/                          # Frontend React app
│   ├── components/              # UI components
│   │   ├── assistant/          # Action system UI
│   │   ├── completion/         # Chat completion UI
│   │   ├── history/            # Conversation history
│   │   ├── settings/           # Settings panels
│   │   ├── speech/             # Speech capture UI
│   │   ├── ui/                 # Reusable UI components (Radix)
│   │   ├── Overlay.tsx         # Screen capture overlay
│   │   └── ...
│   ├── hooks/                   # React hooks
│   │   ├── useSystemAudio.ts   # Audio capture hook
│   │   ├── useCompletion.ts    # Chat completion hook
│   │   ├── useActionAssistant.ts # Action system hook
│   │   └── ...
│   ├── contexts/                # React contexts
│   │   ├── app.context.tsx     # Global app state
│   │   └── theme.context.tsx   # Theme management
│   ├── lib/                     # Utilities
│   │   ├── functions/          # Business logic functions
│   │   ├── database/           # Database operations
│   │   └── storage.ts          # LocalStorage utilities
│   ├── types/                   # TypeScript types
│   ├── config/                  # Configuration constants
│   ├── App.tsx                  # Main app component
│   └── main.tsx                 # React entry point
│
├── src-tauri/                   # Tauri backend
│   ├── src/
│   │   ├── lib.rs              # Application entry
│   │   ├── main.rs             # Binary entry
│   │   ├── api.rs              # HTTP API communication
│   │   ├── activate.rs         # License management
│   │   ├── capture.rs          # Screen capture
│   │   ├── shortcuts.rs        # Global shortcuts
│   │   ├── window.rs           # Window management
│   │   ├── speaker/             # Audio capture
│   │   │   ├── mod.rs
│   │   │   ├── commands.rs
│   │   │   ├── macos.rs
│   │   │   ├── windows.rs
│   │   │   └── linux.rs
│   │   ├── assistant/           # Action system
│   │   │   ├── commands.rs
│   │   │   ├── types.rs
│   │   │   ├── planner/
│   │   │   ├── executor/
│   │   │   ├── validator.rs
│   │   │   ├── policy.rs
│   │   │   └── audit.rs
│   │   └── db/                  # Database migrations
│   │       └── migrations/
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── scribe-api/                  # API server
│   ├── src/
│   │   ├── main.rs             # Server entry
│   │   ├── config.rs           # Configuration
│   │   ├── routes/             # API endpoints
│   │   │   ├── health.rs
│   │   │   ├── auth.rs
│   │   │   ├── chat.rs
│   │   │   ├── audio.rs
│   │   │   ├── models.rs
│   │   │   └── leave.rs
│   │   ├── services/           # Business logic
│   │   │   ├── license.rs
│   │   │   ├── openrouter.rs
│   │   │   ├── whisper.rs
│   │   │   └── leave.rs
│   │   ├── models/             # Data models
│   │   ├── middleware/         # Request middleware
│   │   └── db/                 # Database setup
│   ├── migrations/             # SQL migrations
│   └── Cargo.toml
│
├── package.json                 # Frontend dependencies
├── vite.config.ts              # Vite configuration
├── tsconfig.json               # TypeScript configuration
└── README.md
```

---

## Environment Variables

### Desktop App (Tauri)
- `APP_ENDPOINT`: API server URL (e.g., `https://api.example.com`)
- `API_ACCESS_KEY`: Bearer token for API authentication
- `PAYMENT_ENDPOINT`: Payment/license endpoint URL
- `POSTHOG_API_KEY`: Analytics API key (optional)

### API Server
- `DATABASE_URL`: PostgreSQL connection string
- `LEAVE_DATABASE_URL`: Leave applications database URL
- `OPENAI_API_KEY`: OpenAI API key for Whisper
- `OPENROUTER_API_KEY`: OpenRouter API key
- `API_ACCESS_KEY`: Server authentication key (must match desktop app)
- `PORT`: Server port (default: 3000)

---

## Build & Deployment

### Desktop App
```bash
# Development
npm run dev                    # Start Vite dev server
npm run tauri dev              # Run Tauri app in dev mode

# Production Build
npm run build                  # Build frontend
npm run tauri build            # Build Tauri app

# Output: src-tauri/target/release/bundle/
# - macOS: .dmg files
# - Windows: .msi or .nsis installers
# - Linux: .deb, .rpm, or .AppImage files
```

### API Server
```bash
# Development
cd scribe-api
cargo run

# Production Build
cargo build --release

# Deployment
# - Systemd service
# - Docker container
# - PM2 (ecosystem.config.json provided)
```

---

## Performance Optimizations

### Audio Processing
- Chunked processing (hop_size: 1024 samples)
- Pre-allocated buffers
- Noise gate before VAD (reduces false positives)
- Audio normalization

### Frontend
- React memoization for expensive components
- Lazy loading for heavy components
- Event debouncing (conversation saves: 500ms)
- Efficient state updates (minimal re-renders)

### API
- Connection pooling (SQLx)
- Streaming responses (SSE)
- Async/await throughout
- Database indexing on frequently queried columns

---

## Error Handling

### Frontend
- Try-catch blocks in async functions
- Error state management in hooks
- User-friendly error messages
- Retry logic for network requests
- AbortController for request cancellation

### Tauri
- Result types for all commands
- Error propagation via String errors
- Event-based error notifications
- Graceful degradation

### API Server
- HTTP status codes (200, 400, 401, 500, etc.)
- JSON error responses
- Structured error logging (tracing)
- Database transaction rollback

---

## Future Enhancements

Potential areas for expansion:
- Multi-language support (i18n)
- Plugin system for extensibility
- Cloud sync for conversations
- Advanced action types (network, database, etc.)
- Team collaboration features
- Enhanced analytics and usage tracking
- Voice synthesis (TTS) for responses
- Real-time collaboration
- Mobile app support

---

## Conclusion

Ghost (Scribe) is a sophisticated, privacy-first AI assistant that combines real-time audio capture, AI-powered chat, screen analysis, and action execution in a single desktop application. The architecture is designed for:

- **Privacy**: Local storage, secure credential management
- **Performance**: Streaming responses, efficient audio processing
- **Extensibility**: Custom providers, plugin-ready architecture
- **Cross-Platform**: Native implementations for macOS, Windows, Linux
- **User Experience**: Seamless integration, keyboard shortcuts, minimal UI

The system demonstrates modern desktop application architecture using Tauri, React, and Rust, with a clear separation of concerns and well-defined data flows.
