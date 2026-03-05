-- Migration 7: 3-layer conversation memory
-- Add token_count to messages for caching
ALTER TABLE messages ADD COLUMN token_count INTEGER;

-- Conversation summaries (Layer 2)
CREATE TABLE IF NOT EXISTS conversation_summaries (
  conversation_id TEXT PRIMARY KEY,
  summary_text TEXT NOT NULL,
  last_updated_at INTEGER NOT NULL,
  FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_conversation_summaries_conversation_id ON conversation_summaries(conversation_id);
