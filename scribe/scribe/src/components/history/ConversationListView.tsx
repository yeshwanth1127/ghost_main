import { MessageSquare } from "lucide-react";
import { Button, ScrollArea } from "@/components";
import { UseHistoryType } from "@/hooks/useHistory";
import { ConversationItem } from "./ConversationItem";
import { ChatConversation } from "@/types/completion";

interface ConversationListViewProps extends UseHistoryType {
  currentConversationId: string | null;
  onNewConversation: () => void;
  onClosePopover: () => void;
  onSelectConversation: (conversation: ChatConversation) => void;
}

export const ConversationListView = ({
  conversations,
  currentConversationId,
  selectedConversationId,
  downloadedConversations,
  handleViewConversation,
  onSelectConversation,
  handleDownloadConversation,
  handleDeleteConfirm,
  formatDate,
  onNewConversation,
  onClosePopover,
  setIsOpen,
}: ConversationListViewProps) => {
  const handleNewChat = () => {
    onNewConversation();
    onClosePopover();
  };

  return (
    <>
      <div className="border-b border-input/50 px-4 py-3">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-base font-semibold truncate">
            All Conversations
          </h2>
          <Button
            size="sm"
            onClick={handleNewChat}
            className="shrink-0"
            title="Start new chat"
          >
            New Chat
          </Button>
        </div>
        <p className="text-xs text-muted-foreground mt-0.5">
          Your conversation history
        </p>
      </div>

      <ScrollArea className="h-[min(400px,calc(100vh-10rem))]">
        <div className="p-2">
          {conversations.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-8 text-center">
              <MessageSquare className="h-12 w-12 text-muted-foreground/50 mb-3" />
              <p className="text-sm text-muted-foreground">
                No conversations yet
              </p>
              <p className="text-xs text-muted-foreground/70 mt-1">
                Start chatting to see your history here
              </p>
            </div>
          ) : (
            <div className="space-y-1 pr-2">
              {conversations.map((conversation) => (
                <ConversationItem
                  {...conversation}
                  conversation={conversation}
                  currentConversationId={currentConversationId}
                  {...{
                    conversations,
                    selectedConversationId,
                    downloadedConversations,
                    handleViewConversation,
                    onSelectConversation,
                    handleDownloadConversation,
                    handleDeleteConfirm,
                    formatDate,
                    isOpen: false,
                    viewingConversation: null,
                    deleteConfirm: null,
                    setIsOpen,
                    confirmDelete: () => {},
                    cancelDelete: () => {},
                    refreshConversations: () => {},
                  }}
                />
              ))}
            </div>
          )}
        </div>
      </ScrollArea>
    </>
  );
};
