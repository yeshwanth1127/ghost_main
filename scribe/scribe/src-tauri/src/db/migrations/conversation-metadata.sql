-- Conversation metadata and semantic memory (context improvements)
-- Migration 6: add model_used, total_tokens to conversations
ALTER TABLE conversations ADD COLUMN model_used TEXT;
ALTER TABLE conversations ADD COLUMN total_tokens INTEGER;

-- Semantic memory: key-value facts per conversation (e.g. "api_key", "project_path")
CREATE TABLE IF NOT EXISTS conversation_facts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_conversation_facts_conversation_id ON conversation_facts(conversation_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_conversation_facts_conv_key ON conversation_facts(conversation_id, key);
