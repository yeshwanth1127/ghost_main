import { Loader2, Send } from "lucide-react";
import {
  Button,
  Input as InputComponent,
} from "@/components";
import { MessageHistory } from "../history";
import { UseCompletionReturn } from "@/types";

export const Input = ({
  isLoading,
  input,
  setInput,
  handleKeyPress,
  handlePaste,
  currentConversationId,
  conversationHistory,
  startNewConversation,
  messageHistoryOpen,
  setMessageHistoryOpen,
  inputRef,
  isHidden,
  submit,
}: UseCompletionReturn & { isHidden: boolean }) => {
  return (
    <div className="select-none flex items-center gap-2 w-full min-w-0">
      <div className="relative flex-1 min-w-0">
        <InputComponent
          ref={inputRef}
          placeholder="Ask me anything..."
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyPress}
          onPaste={handlePaste}
          disabled={isLoading || isHidden}
          className={`w-full ${
            currentConversationId && conversationHistory.length > 0
              ? "pr-14"
              : "pr-2"
          }`}
        />

        {/* Conversation thread indicator */}
        {currentConversationId &&
          conversationHistory.length > 0 &&
          !isLoading && (
            <div className="absolute select-none right-1 top-1/2 -translate-y-1/2 flex items-center gap-1">
              <MessageHistory
                conversationHistory={conversationHistory}
                currentConversationId={currentConversationId}
                onStartNewConversation={startNewConversation}
                messageHistoryOpen={messageHistoryOpen}
                setMessageHistoryOpen={setMessageHistoryOpen}
              />
            </div>
          )}

        {/* Loading indicator */}
        {isLoading && (
          <div className="absolute right-3 top-1/2 -translate-y-1/2 animate-pulse">
            <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
          </div>
        )}
      </div>
      <Button
        type="button"
        size="icon"
        variant="default"
        onClick={() => {
          if (!isLoading && input.trim()) {
            submit();
          }
        }}
        disabled={isLoading || isHidden || !input.trim()}
        className="shrink-0 h-9 w-9 rounded-lg"
        title="Send"
      >
        <Send className="h-4 w-4" />
      </Button>
    </div>
  );
};
