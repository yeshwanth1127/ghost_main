import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { MessageCircleQuestion, X } from "lucide-react";

interface AskUserDialogProps {
  runId: string;
  requestId: string;
  question: string;
  reason?: string;
  onClose: () => void;
}

export const AskUserDialog: React.FC<AskUserDialogProps> = ({
  requestId,
  question,
  reason,
  onClose,
}) => {
  const [answer, setAnswer] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async () => {
    if (!answer.trim()) return;
    setIsSubmitting(true);
    try {
      await invoke("reply_ask_user", {
        requestId,
        answer: answer.trim(),
      });
      onClose();
    } catch (error) {
      console.error("Failed to submit clarification:", error);
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
      <div className="w-full max-w-md rounded-lg border bg-background p-4 shadow-lg">
        <div className="mb-3 flex items-center gap-2 text-sm font-medium">
          <MessageCircleQuestion className="h-4 w-4 text-muted-foreground" />
          Clarification needed
        </div>
        <p className="mb-3 text-sm text-foreground">{question}</p>
        {reason && (
          <p className="mb-3 text-xs text-muted-foreground">Reason: {reason}</p>
        )}
        <Textarea
          value={answer}
          onChange={(e) => setAnswer(e.target.value)}
          placeholder="Your answer..."
          className="mb-3 min-h-[80px] resize-y"
          disabled={isSubmitting}
        />
        <div className="flex justify-end gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={async () => {
              try {
                await invoke("reply_ask_user", { requestId, answer: "" });
              } catch (_) {}
              onClose();
            }}
            disabled={isSubmitting}
          >
            <X className="h-4 w-4 mr-1" />
            Cancel
          </Button>
          <Button size="sm" onClick={handleSubmit} disabled={!answer.trim() || isSubmitting}>
            Send
          </Button>
        </div>
      </div>
    </div>
  );
};
