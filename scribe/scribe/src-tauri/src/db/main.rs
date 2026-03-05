use tauri_plugin_sql::{Migration, MigrationKind};

/// Returns all database migrations
pub fn migrations() -> Vec<Migration> {
    vec![
        // Migration 1: Create system_prompts table with indexes and triggers
        Migration {
            version: 1,
            description: "create_system_prompts_table",
            sql: include_str!("migrations/system-prompts.sql"),
            kind: MigrationKind::Up,
        },
        // Migration 2: Create chat history tables (conversations and messages)
        Migration {
            version: 2,
            description: "create_chat_history_tables",
            sql: include_str!("migrations/chat-history.sql"),
            kind: MigrationKind::Up,
        },
        // Migration 3: Create assistant tables (action_snapshots, audit_logs, capability_tokens)
        Migration {
            version: 3,
            description: "create_assistant_tables",
            sql: include_str!("migrations/assistant.sql"),
            kind: MigrationKind::Up,
        },
        // Migration 4: Create agent runs and run_events tables
        Migration {
            version: 4,
            description: "create_agent_runs_tables",
            sql: include_str!("migrations/agent-runs.sql"),
            kind: MigrationKind::Up,
        },
        // Migration 5: Create execution_tickets table (Moltbot-style tool execution lifecycle)
        Migration {
            version: 5,
            description: "create_execution_tickets_table",
            sql: include_str!("migrations/execution-tickets.sql"),
            kind: MigrationKind::Up,
        },
        // Migration 6: Conversation metadata (model_used, total_tokens) and conversation_facts (semantic memory)
        Migration {
            version: 6,
            description: "conversation_metadata_and_facts",
            sql: include_str!("migrations/conversation-metadata.sql"),
            kind: MigrationKind::Up,
        },
    ]
}

