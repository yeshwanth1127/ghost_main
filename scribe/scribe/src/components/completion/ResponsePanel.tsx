import { Loader2, XIcon } from "lucide-react";
import {
  Button,
  ScrollArea,
  Markdown,
  Switch,
} from "@/components";
import { UseCompletionReturn } from "@/types";
import { CopyButton } from "../Markdown/copy-button";

export const ResponsePanel = ({
  isLoading,
  reset,
  inputRef,
  response,
  cancel,
  keepEngaged,
  setKeepEngaged,
  startNewConversation,
}: UseCompletionReturn) => {
  return (
    <div className="flex items-center justify-between px-4 py-2 border-b bg-muted/30">
      <div className="flex flex-row gap-1 items-center">
        <h3 className="font-semibold text-sm select-none">
          {keepEngaged ? "Conversation Mode" : "AI Response"}
        </h3>
        <div className="text-xs text-muted-foreground/70">
          (Use arrow keys to scroll)
        </div>
      </div>
      <div className="flex items-center gap-2 select-none">
        <div className="flex flex-row items-center gap-2 mr-2">
          <p className="text-sm">{`Toggle ${
            keepEngaged ? "AI response" : "conversation mode"
          }`}</p>
          <span className="text-xs text-muted-foreground/60 bg-muted/30 px-1 py-0 rounded border border-input/50">
            {navigator.platform.toLowerCase().includes("mac")
              ? "⌘"
              : "Ctrl"}{" "}
            + K
          </span>
          <Switch
            checked={keepEngaged}
            onCheckedChange={(checked) => {
              setKeepEngaged(checked);
              setTimeout(() => {
                inputRef?.current?.focus();
              }, 100);
            }}
          />
        </div>
        <CopyButton content={response} />
        <Button
          size="icon"
          variant="ghost"
          onClick={() => {
            if (isLoading) {
              cancel();
            } else if (keepEngaged) {
              setKeepEngaged(false);
              startNewConversation();
            } else {
              reset();
            }
          }}
          className="cursor-pointer"
          title={
            isLoading
              ? "Cancel loading"
              : keepEngaged
              ? "Close and start new conversation"
              : "Clear conversation"
          }
        >
          <XIcon />
        </Button>
      </div>
    </div>
  );
};

export const ResponsePanelContent = ({
  isLoading,
  error,
  response,
  conversationHistory,
  keepEngaged,
  scrollAreaRef,
}: UseCompletionReturn) => {
  console.log("[ResponsePanelContent] Rendering with:", {
    isLoading,
    hasError: !!error,
    responseLength: response?.length || 0,
    responsePreview: response?.slice(0, 50),
    conversationHistoryLength: conversationHistory?.length || 0,
  });

  return (
    <ScrollArea
      ref={scrollAreaRef}
      className="h-[min(520px,calc(100vh-14rem))] max-h-[calc(100vh-14rem)]"
    >
      <div className="p-4">
        {error && (
          <div className="mb-4 p-3 bg-destructive/10 border border-destructive/20 rounded text-sm text-destructive">
            <strong>Error:</strong> {error}
          </div>
        )}
        {isLoading && (
          <div className="flex items-center gap-2 my-4 text-muted-foreground animate-pulse select-none">
            <Loader2 className="h-4 w-4 animate-spin" />
            <span className="text-sm">Generating response...</span>
          </div>
        )}
        {response && <Markdown>{response}</Markdown>}

        {keepEngaged && conversationHistory.length > 1 && (
          <div className="space-y-3 pt-3">
            {conversationHistory
              .sort((a, b) => b?.timestamp - a?.timestamp)
              .map((message, index) => {
                if (!isLoading && index === 0) {
                  return null;
                }
                return (
                  <div
                    key={message.id}
                    className={`p-3 rounded-lg text-sm ${
                      message.role === "user"
                        ? "bg-primary/10 border-l-4 border-primary"
                        : "bg-muted/50"
                    }`}
                  >
                    <div className="flex items-center gap-2 mb-2">
                      <span className="text-xs font-medium text-muted-foreground uppercase">
                        {message.role === "user" ? "You" : "AI"}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        {new Date(message.timestamp).toLocaleTimeString(
                          [],
                          {
                            hour: "2-digit",
                            minute: "2-digit",
                          }
                        )}
                      </span>
                    </div>
                    <Markdown>{message.content}</Markdown>
                  </div>
                );
              })}
          </div>
        )}
      </div>
    </ScrollArea>
  );
};
