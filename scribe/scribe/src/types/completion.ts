// Completion-related types
export interface AttachedFile {
  id: string;
  name: string;
  type: string;
  base64: string;
  size: number;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: number;
  attachedFiles?: AttachedFile[];
  /** Cached token count (computed at save, avoids re-tokenizing) */
  token_count?: number;
}

export interface ChatConversation {
  id: string;
  title: string;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
  /** Model used for this conversation (e.g. "ollama/llama3") */
  modelUsed?: string | null;
  /** Total tokens used (if tracked) */
  totalTokens?: number | null;
}

export interface CompletionState {
  input: string;
  response: string;
  isLoading: boolean;
  error: string | null;
  attachedFiles: AttachedFile[];
  currentConversationId: string | null;
  conversationHistory: ChatMessage[];
}

// Provider-related types
export interface Message {
  role: "system" | "user" | "assistant";
  content:
    | string
    | Array<{
        type: string;
        text?: string;
        image_url?: { url: string };
        source?: any;
        inline_data?: any;
      }>;
}
