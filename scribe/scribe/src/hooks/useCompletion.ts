import { useState, useCallback, useRef, useEffect } from "react";
import { useWindowResize } from "./useWindow";
import { useGlobalShortcuts } from "@/hooks";
import { DEFAULT_SYSTEM_PROMPT, MAX_FILES } from "@/config";
import { GHOST_CAPABILITIES_DISPLAY } from "@/config/ghost-capabilities";
import { useApp } from "@/contexts";
import {
  fetchAIResponse,
  saveConversation,
  stripReasoningFromContent,
  getConversationById,
  generateConversationTitle,
  shouldUseScribeAPI,
  MESSAGE_ID_OFFSET,
  generateConversationId,
  generateMessageId,
  generateRequestId,
  trimHistoryToContextWindow,
  getFactsForConversation,
  isScreenContentQuery,
} from "@/lib";
import { recordUsageFromClient } from "@/lib/usage-api";
import { CHARS_PER_TOKEN_ESTIMATE } from "@/lib/chat-constants";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useActionAssistant } from "./useActionAssistant";

// Types for completion
interface AttachedFile {
  id: string;
  name: string;
  type: string;
  base64: string;
  size: number;
}

interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: number;
}

interface ChatConversation {
  id: string;
  title: string;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
}

interface CompletionState {
  input: string;
  response: string;
  isLoading: boolean;
  error: string | null;
  attachedFiles: AttachedFile[];
  currentConversationId: string | null;
  conversationHistory: ChatMessage[];
}

interface LeaveApplicationEventDetail {
  formValues: {
    name: string;
    usn: string;
    department: string;
    reason: string;
  };
  attachments?: AttachedFile[];
}

interface LeaveApplicationContext {
  formValues: LeaveApplicationEventDetail["formValues"];
  attachments: AttachedFile[];
}

/** Normalize API errors to user-friendly messages (e.g. usage limit) */
function normalizeErrorMessage(raw: string): string {
  const lower = raw.toLowerCase();
  if (
    lower.includes("token limit exceeded") ||
    lower.includes("usage limit") ||
    lower.includes("402")
  ) {
    return "You reached your monthly AI usage limit.";
  }
  return raw;
}

export const useCompletion = () => {
  const {
    selectedAIProvider,
    allAiProviders,
    systemPrompt,
    screenshotConfiguration,
    setScreenshotConfiguration,
  } = useApp();
  const globalShortcuts = useGlobalShortcuts();
  const actionAssistant = useActionAssistant();
  const effectiveSystemPrompt =
    systemPrompt &&
    systemPrompt.includes(
      "You are an autonomous agent making decisions about what action to take next."
    )
      ? DEFAULT_SYSTEM_PROMPT
      : systemPrompt;

  const [state, setState] = useState<CompletionState>({
    input: "",
    response: "",
    isLoading: false,
    error: null,
    attachedFiles: [],
    currentConversationId: null,
    conversationHistory: [],
  });
  const [micOpen, setMicOpen] = useState(false);
  const [enableVAD, setEnableVAD] = useState(false);
  const [messageHistoryOpen, setMessageHistoryOpen] = useState(false);
  const [isFilesPopoverOpen, setIsFilesPopoverOpen] = useState(false);
  const [isScreenshotLoading, setIsScreenshotLoading] = useState(false);
  const [keepEngaged, setKeepEngaged] = useState(false);
  const [responseOpen, setResponseOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const isProcessingScreenshotRef = useRef(false);
  const screenshotConfigRef = useRef(screenshotConfiguration);
  const hasCheckedPermissionRef = useRef(false);
  const leaveApplicationRef = useRef<LeaveApplicationContext | null>(null);

  const { resizeWindow } = useWindowResize();

  useEffect(() => {
    screenshotConfigRef.current = screenshotConfiguration;
  }, [screenshotConfiguration]);

  const scrollAreaRef = useRef<HTMLDivElement>(null);

  const abortControllerRef = useRef<AbortController | null>(null);
  const currentRequestIdRef = useRef<string | null>(null);

  const setInput = useCallback((value: string) => {
    setState((prev) => ({ ...prev, input: value }));
  }, []);

  const setResponse = useCallback((value: string) => {
    setState((prev) => ({ ...prev, response: value }));
  }, []);

  const addFile = useCallback(async (file: File) => {
    try {
      const base64 = await fileToBase64(file);
      const attachedFile: AttachedFile = {
        id: Date.now().toString(),
        name: file.name,
        type: file.type,
        base64,
        size: file.size,
      };

      setState((prev) => ({
        ...prev,
        attachedFiles: [...prev.attachedFiles, attachedFile],
      }));
    } catch (error) {
      console.error("Failed to process file:", error);
    }
  }, []);

  const removeFile = useCallback((fileId: string) => {
    setState((prev) => ({
      ...prev,
      attachedFiles: prev.attachedFiles.filter((f) => f.id !== fileId),
    }));
  }, []);

  const clearFiles = useCallback(() => {
    setState((prev) => ({ ...prev, attachedFiles: [] }));
  }, []);

  const submitLeaveApplicationToApi = useCallback(
    async (context: LeaveApplicationContext, summary: string) => {
      const trimmedSummary = summary.trim();
      if (!trimmedSummary) {
        return;
      }

      try {
        console.log(
          "[Leave] Submitting application to API",
          context.formValues.name,
          context.formValues.usn,
          `attachments=${context.attachments.length}`
        );
        await invoke("submit_leave_application", {
          payload: {
            name: context.formValues.name,
            usn: context.formValues.usn,
            department: context.formValues.department,
            reason: context.formValues.reason,
            summary: trimmedSummary,
            attachments: context.attachments.map((attachment) => ({
              id: attachment.id,
              name: attachment.name,
              type: attachment.type,
              base64: attachment.base64,
              size: Math.round(attachment.size),
            })),
          },
        });
        console.log("[Leave] Leave application stored successfully");
      } catch (error) {
        console.error("Failed to persist leave application:", error);
      }
    },
    []
  );

  /** Persist user message immediately so we don't lose the turn on crash/close. Returns conversationId used. */
  const saveUserMessageOnly = useCallback(
    async (userMessage: string): Promise<string> => {
      if (!userMessage?.trim()) return state.currentConversationId ?? "";
      const conversationId =
        state.currentConversationId || generateConversationId("chat");
      const timestamp = Date.now();
      const userMsg: ChatMessage = {
        id: generateMessageId("user", timestamp),
        role: "user",
        content: userMessage.trim(),
        timestamp,
      };
      const newMessages = [...state.conversationHistory, userMsg];
      let existingConversation: ChatConversation | null = null;
      if (state.currentConversationId) {
        try {
          existingConversation = await getConversationById(
            state.currentConversationId
          );
        } catch {
          // ignore
        }
      }
      const title =
        state.conversationHistory.length === 0
          ? generateConversationTitle(userMessage)
          : existingConversation?.title ?? generateConversationTitle(userMessage);
      const conversation: ChatConversation = {
        id: conversationId,
        title,
        messages: newMessages,
        createdAt: existingConversation?.createdAt ?? timestamp,
        updatedAt: timestamp,
      };
      try {
        await saveConversation(conversation);
        setState((prev) => ({
          ...prev,
          currentConversationId: conversationId,
        }));
        return conversationId;
      } catch (err) {
        console.error("Failed to save user message:", err);
        return conversationId;
      }
    },
    [state.currentConversationId, state.conversationHistory]
  );

  const submit = useCallback(
    async (
      options?:
        | string
        | {
            speechText?: string;
            inputOverride?: string;
            attachmentsOverride?: AttachedFile[];
          }
    ) => {
      const speechText =
        typeof options === "string" ? options : options?.speechText;
      const inputOverride =
        typeof options === "object" && options !== null
          ? options.inputOverride
          : undefined;
      const attachmentsOverride =
        typeof options === "object" && options !== null
          ? options.attachmentsOverride
          : undefined;

      const input = inputOverride ?? speechText ?? state.input;

      if (!input.trim()) {
        return;
      }
      setResponseOpen(true);

      if (speechText && !inputOverride) {
        setState((prev) => ({
          ...prev,
          input: speechText,
        }));
      } else if (inputOverride) {
        setState((prev) => ({
          ...prev,
          input: inputOverride,
        }));
      }

      const attachments = attachmentsOverride ?? state.attachedFiles;

      if (attachmentsOverride) {
        setState((prev) => ({
          ...prev,
          attachedFiles: attachmentsOverride,
        }));
      }

      // Check if input is an action request (simple pattern matching)
      const actionKeywords = [
        "create file",
        "read file",
        "copy file",
        "move file",
        "delete file",
        "create directory",
        "create folder",
        "mkdir",
        "rename",
      ];
      const isActionRequest = actionKeywords.some((keyword) =>
        input.toLowerCase().includes(keyword.toLowerCase())
      );

      // If it's an action request, try to parse it
      if (isActionRequest) {
        try {
          const plan = await actionAssistant.parseIntent(input);
          const preview = await actionAssistant.previewAction(plan);
          
          // Emit event to show action preview (UI will handle this)
          // For now, we'll proceed with normal chat but this could trigger preview UI
          // This is a simplified integration - full implementation would show preview modal
          console.log("Action detected, preview:", preview);
          // You could emit an event here or set state to show preview
        } catch (error) {
          // If parsing fails, fall through to normal chat
          console.log("Action parsing failed, proceeding with chat:", error);
        }
      }

      // Generate unique request ID
      const requestId = generateRequestId();
      currentRequestIdRef.current = requestId;

      // Cancel any existing request
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
      }

      abortControllerRef.current = new AbortController();
      const signal = abortControllerRef.current.signal;

      try {
        const useScribeAPI = await shouldUseScribeAPI(selectedAIProvider);
        // Check if AI provider is configured
        if (!selectedAIProvider.provider && !useScribeAPI) {
          setState((prev) => ({
            ...prev,
            error: "Please select an AI provider in settings",
          }));
          return;
        }

        const provider = allAiProviders.find(
          (p) => p.id === selectedAIProvider.provider
        );
        if (!provider && !useScribeAPI) {
          setState((prev) => ({
            ...prev,
            error: "Invalid provider selected",
          }));
          return;
        }

        // Save user message immediately so we don't lose the turn on crash/close
        const conversationIdUsed = await saveUserMessageOnly(input);

        // Context window: trim history to message count + token budget
        const trimmedHistory = trimHistoryToContextWindow(state.conversationHistory);
        const messageHistory = trimmedHistory.map((msg) => ({
          role: msg.role,
          content: msg.content,
        }));

        // Semantic memory: inject stored facts into system prompt
        const facts =
          conversationIdUsed ? await getFactsForConversation(conversationIdUsed) : {};
        const factsEntries = Object.entries(facts);
        const systemPromptWithFacts =
          factsEntries.length > 0
            ? (effectiveSystemPrompt || "") +
              "\n\nStored facts for this conversation: " +
              factsEntries.map(([k, v]) => `${k}: ${v}`).join("; ")
            : effectiveSystemPrompt || undefined;

        // Handle image attachments
        const imagesBase64: string[] = [];

        // When user asks about screen content (e.g. "answer the question on my screen"),
        // auto-capture what's behind Ghost (hides Ghost, captures, restores) so the AI sees
        // the content (e.g. Cursor, a test) rather than Ghost itself.
        if (isScreenContentQuery(input)) {
          try {
            const screenBase64 = await invoke<string>("capture_screen_behind_ghost");
            if (screenBase64) {
              imagesBase64.push(screenBase64);
            }
          } catch (e) {
            console.warn("[useCompletion] Screen capture failed:", e);
          }
        }

        if (attachments.length > 0) {
          attachments.forEach((file) => {
            if (file.type.startsWith("image/")) {
              imagesBase64.push(file.base64);
            }
          });
        }

        let fullResponse = "";

        // Clear previous response and set loading state
        setState((prev) => ({
          ...prev,
          isLoading: true,
          error: null,
          response: "",
        }));

        try {
          console.log("[useCompletion] submit", {
            inputLength: input.length,
            attachments: attachments.length,
            useScribeAPI,
            provider: useScribeAPI ? "scribe" : provider?.id,
            conversationId: conversationIdUsed,
          });
          // Use the fetchAIResponse function with signal
          for await (const chunk of fetchAIResponse({
            provider: useScribeAPI ? undefined : provider,
            selectedProvider: selectedAIProvider,
            systemPrompt: systemPromptWithFacts,
            history: messageHistory,
            userMessage: input,
            imagesBase64,
            signal,
          })) {
            console.log("[useCompletion] ✅ Received chunk from generator:", {
              length: chunk.length,
              preview: chunk.slice(0, 80),
              currentResponseLength: fullResponse.length,
            });
            // Only update if this is still the current request
            if (currentRequestIdRef.current !== requestId) {
              return; // Request was superseded, stop processing
            }

            // Check if request was aborted
            if (signal.aborted) {
              return; // Request was cancelled, stop processing
            }

            fullResponse += chunk;
            setState((prev) => {
              const accumulated = prev.response + chunk;
              const stripped = stripReasoningFromContent(accumulated);
              console.log("[useCompletion] 📝 Updating UI state:", {
                accumulatedLength: accumulated.length,
                strippedLength: stripped.length,
                chunkLength: chunk.length,
                newResponse: stripped.slice(0, 50),
                prevResponse: prev.response.slice(0, 50),
              });
              return {
                ...prev,
                response: stripped,
              };
            });
          }
        } catch (e: any) {
          // Only show error if this is still the current request and not aborted
          if (currentRequestIdRef.current === requestId && !signal.aborted) {
            console.error("[useCompletion] Error in fetchAIResponse:", e);
            const raw = e?.message || e?.toString() || "An error occurred";
            setState((prev) => ({
              ...prev,
              isLoading: false,
              error: normalizeErrorMessage(raw),
              response: "", // Clear response on error
            }));
          }
          return;
        }

        // Only proceed if this is still the current request
        if (currentRequestIdRef.current !== requestId || signal.aborted) {
          return;
        }

        setState((prev) => {
          console.log("[useCompletion] Before setting isLoading=false:", {
            currentResponse: prev.response.slice(0, 80),
            currentResponseLength: prev.response.length,
          });
          return { ...prev, isLoading: false };
        });
        console.log("[useCompletion] completed", {
          fullResponseLength: fullResponse.length,
          fullResponsePreview: fullResponse.slice(0, 80),
        });

        // Don't focus input here - it steals focus from the response panel and can
        // cause the popover to close, clearing the response immediately

        // Save the conversation after successful completion (use stripped response)
        if (fullResponse) {
          const cleanedResponse = stripReasoningFromContent(fullResponse) || fullResponse;
          await saveCurrentConversation(
            input,
            cleanedResponse,
            attachments
          );

          // Record usage for direct providers (Scribe API records its own)
          if (!useScribeAPI && selectedAIProvider?.provider) {
            try {
              const storage = await invoke<{ license_key?: string }>("secure_storage_get");
              if (storage?.license_key) {
                const promptChars =
                  (systemPromptWithFacts?.length || 0) +
                  messageHistory.reduce((sum, m) => sum + (m.content?.length || 0), 0) +
                  input.length;
                const promptTokens = Math.max(1, Math.ceil(promptChars / CHARS_PER_TOKEN_ESTIMATE));
                const completionTokens = Math.max(1, Math.ceil(cleanedResponse.length / CHARS_PER_TOKEN_ESTIMATE));
                const model = selectedAIProvider.variables?.model || "unknown";
                const provider = selectedAIProvider.provider || "exora";
                console.log("[useCompletion] Recording usage:", {
                  provider,
                  model,
                  promptTokens,
                  completionTokens,
                  promptChars,
                  responseChars: cleanedResponse.length,
                });
                await recordUsageFromClient(
                  storage.license_key,
                  model,
                  provider,
                  promptTokens,
                  completionTokens
                );
              } else {
                console.warn("[useCompletion] No license key - usage not recorded. Register to track usage.");
              }
            } catch (e) {
              console.warn("[useCompletion] Failed to record usage:", e);
            }
          }

          const context = leaveApplicationRef.current;
          if (context) {
            await submitLeaveApplicationToApi(context, cleanedResponse);
            leaveApplicationRef.current = null;
          }

          // Clear input and attached files after saving
          setState((prev) => ({
            ...prev,
            input: "",
            attachedFiles: [],
          }));
        }
      } catch (error) {
        leaveApplicationRef.current = null;
        // Only show error if not aborted
        if (!signal?.aborted && currentRequestIdRef.current === requestId) {
          const raw = error instanceof Error ? error.message : "An error occurred";
          setState((prev) => ({
            ...prev,
            error: normalizeErrorMessage(raw),
            isLoading: false,
          }));
        }
      }
    },
    [
      state.input,
      state.attachedFiles,
      selectedAIProvider,
      allAiProviders,
      effectiveSystemPrompt,
      state.conversationHistory,
      submitLeaveApplicationToApi,
      saveUserMessageOnly,
    ]
  );

  const cancel = useCallback(() => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
      abortControllerRef.current = null;
    }
    currentRequestIdRef.current = null;
    setResponseOpen(false);
    setState((prev) => ({ ...prev, isLoading: false }));
  }, []);

  const reset = useCallback(() => {
    // Don't reset if keep engaged mode is active
    if (keepEngaged) {
      return;
    }
    cancel();
    setResponseOpen(false);
    setState((prev) => ({
      ...prev,
      input: "",
      response: "",
      error: null,
      attachedFiles: [],
    }));
  }, [cancel, keepEngaged]);

  // Helper function to convert file to base64
  const fileToBase64 = useCallback(async (file: File): Promise<string> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.readAsDataURL(file);
      reader.onload = () => {
        const base64 = (reader.result as string)?.split(",")[1] || "";
        resolve(base64);
      };
      reader.onerror = reject;
    });
  }, []);

  const loadConversation = useCallback((conversation: ChatConversation) => {
    setState((prev) => ({
      ...prev,
      currentConversationId: conversation.id,
      conversationHistory: conversation.messages,
      input: "",
      response: "",
      error: null,
      isLoading: false,
    }));
  }, []);

  const startNewConversation = useCallback(() => {
    setState((prev) => ({
      ...prev,
      currentConversationId: null,
      conversationHistory: [],
      input: "",
      response: "",
      error: null,
      isLoading: false,
      attachedFiles: [],
    }));
  }, []);

  const saveCurrentConversation = useCallback(
    async (
      userMessage: string,
      assistantResponse: string,
      _attachedFiles: AttachedFile[]
    ) => {
      // Validate inputs
      if (!userMessage || !assistantResponse) {
        console.error("Cannot save conversation: missing message content");
        return;
      }

      const conversationId =
        state.currentConversationId || generateConversationId("chat");
      const timestamp = Date.now();

      const userMsg: ChatMessage = {
        id: generateMessageId("user", timestamp),
        role: "user",
        content: userMessage,
        timestamp,
      };

      const assistantMsg: ChatMessage = {
        id: generateMessageId("assistant", timestamp + MESSAGE_ID_OFFSET),
        role: "assistant",
        content: assistantResponse,
        timestamp: timestamp + MESSAGE_ID_OFFSET,
      };

      const newMessages = [...state.conversationHistory, userMsg, assistantMsg];

      // Get existing conversation if updating
      let existingConversation = null;
      if (state.currentConversationId) {
        try {
          existingConversation = await getConversationById(
            state.currentConversationId
          );
        } catch (error) {
          console.error("Failed to get existing conversation:", error);
        }
      }

      const title =
        state.conversationHistory.length === 0
          ? generateConversationTitle(userMessage)
          : existingConversation?.title ||
            generateConversationTitle(userMessage);

      const modelUsed =
        selectedAIProvider?.provider != null
          ? `${selectedAIProvider.provider}/${selectedAIProvider?.variables?.model ?? "default"}`
          : undefined;

      const conversation: ChatConversation = {
        id: conversationId,
        title,
        messages: newMessages,
        createdAt: existingConversation?.createdAt || timestamp,
        updatedAt: timestamp,
        modelUsed: modelUsed ?? null,
      };

      try {
        await saveConversation(conversation);

        setState((prev) => ({
          ...prev,
          currentConversationId: conversationId,
          conversationHistory: newMessages,
        }));
      } catch (error) {
        console.error("Failed to save conversation:", error);
        // Show error to user
        setState((prev) => ({
          ...prev,
          error: "Failed to save conversation. Please try again.",
        }));
      }
    },
    [
      state.currentConversationId,
      state.conversationHistory,
      selectedAIProvider,
    ]
  );

  const showCapabilities = useCallback(async () => {
    const conversationId =
      state.currentConversationId || generateConversationId("chat");
    const timestamp = Date.now();
    const userMsg: ChatMessage = {
      id: generateMessageId("user", timestamp),
      role: "user",
      content: "What can Ghost do?",
      timestamp,
    };
    const assistantMsg: ChatMessage = {
      id: generateMessageId("assistant", timestamp + MESSAGE_ID_OFFSET),
      role: "assistant",
      content: GHOST_CAPABILITIES_DISPLAY,
      timestamp: timestamp + MESSAGE_ID_OFFSET,
    };
    const newMessages = [...state.conversationHistory, userMsg, assistantMsg];
    let existingConversation = null;
    if (state.currentConversationId) {
      try {
        existingConversation = await getConversationById(
          state.currentConversationId
        );
      } catch {
        // ignore
      }
    }
    const title =
      state.conversationHistory.length === 0
        ? "What can Ghost do?"
        : existingConversation?.title ?? "What can Ghost do?";
    const conversation: ChatConversation = {
      id: conversationId,
      title,
      messages: newMessages,
      createdAt: existingConversation?.createdAt ?? timestamp,
      updatedAt: timestamp,
    };
    try {
      await saveConversation(conversation);
      setState((prev) => ({
        ...prev,
        currentConversationId: conversationId,
        conversationHistory: newMessages,
        response: "",
        error: null,
      }));
    } catch (error) {
      console.error("Failed to save capabilities message:", error);
      setState((prev) => ({
        ...prev,
        error: "Failed to show capabilities. Please try again.",
      }));
    }
  }, [state.currentConversationId, state.conversationHistory]);

  // Listen for conversation events from the main ChatHistory component
  useEffect(() => {
    const handleConversationSelected = async (event: any) => {
      // Only the conversation ID is passed through the event
      const { id } = event.detail;

      if (!id || typeof id !== "string") {
        console.error("No conversation ID provided");
        setState((prev) => ({
          ...prev,
          error: "Invalid conversation selected",
        }));
        return;
      }

      try {
        // Fetch the full conversation from SQLite
        const conversation = await getConversationById(id);

        if (conversation) {
          loadConversation(conversation);
        } else {
          console.error(`Conversation ${id} not found in database`);
          setState((prev) => ({
            ...prev,
            error: "Conversation not found. It may have been deleted.",
          }));
        }
      } catch (error) {
        console.error("Failed to load conversation:", error);
        setState((prev) => ({
          ...prev,
          error: "Failed to load conversation. Please try again.",
        }));
      }
    };

    const handleNewConversation = () => {
      startNewConversation();
    };

    const handleConversationDeleted = (event: any) => {
      const deletedId = event.detail;
      // If the currently active conversation was deleted, start a new one
      if (state.currentConversationId === deletedId) {
        startNewConversation();
      }
    };

    window.addEventListener("conversationSelected", handleConversationSelected);
    window.addEventListener("newConversation", handleNewConversation);
    window.addEventListener("conversationDeleted", handleConversationDeleted);

    return () => {
      window.removeEventListener(
        "conversationSelected",
        handleConversationSelected
      );
      window.removeEventListener("newConversation", handleNewConversation);
      window.removeEventListener(
        "conversationDeleted",
        handleConversationDeleted
      );
    };
  }, [loadConversation, startNewConversation, state.currentConversationId]);

  useEffect(() => {
    const handleLeaveApplicationSubmitted = (
      event: CustomEvent<LeaveApplicationEventDetail>
    ) => {
      if (!event?.detail) {
        return;
      }

      const { formValues, attachments = [] } = event.detail;
      if (!formValues) {
        return;
      }

      const name = formValues.name?.trim() || "N/A";
      const usn = formValues.usn?.trim() || "N/A";
      const department = formValues.department?.trim() || "N/A";
      const reason = formValues.reason?.trim() || "N/A";
      const normalizedAttachments = attachments.map((file) => ({ ...file }));

      leaveApplicationRef.current = {
        formValues: { name, usn, department, reason },
        attachments: normalizedAttachments,
      };

      const promptInstructions =
        "You are a leave analyzer for a college. Read the details and any attached documents, then draft a concise summary of this leave request. then show if its valid or not valid";

      const structuredDetails = [
        "Leave application details:",
        `- Name: ${name}`,
        `- USN: ${usn}`,
        `- Department: ${department}`,
        `- Attachments Provided: ${attachments.length}`,
        "",
        "Reason Provided:",
        reason,
      ].join("\n");

      const combinedPrompt = `${promptInstructions}\n\n${structuredDetails}`;

      startNewConversation();

      submit({
        inputOverride: combinedPrompt,
        attachmentsOverride: normalizedAttachments,
      });
    };

    const listener = (event: Event) =>
      handleLeaveApplicationSubmitted(
        event as CustomEvent<LeaveApplicationEventDetail>
      );

    window.addEventListener("leaveApplicationSubmitted", listener);

    return () => {
      window.removeEventListener("leaveApplicationSubmitted", listener);
    };
  }, [submit, startNewConversation]);

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    const MAX_FILES = 6;

    files.forEach((file) => {
      if (
        file.type.startsWith("image/") &&
        state.attachedFiles.length < MAX_FILES
      ) {
        addFile(file);
      }
    });

    // Reset input so same file can be selected again
    e.target.value = "";
  };

  const handleScreenshotSubmit = useCallback(
    async (base64: string, prompt?: string) => {
      if (state.attachedFiles.length >= MAX_FILES) {
        setState((prev) => ({
          ...prev,
          error: `You can only upload ${MAX_FILES} files`,
        }));
        return;
      }

      try {
        if (prompt) {
          // Auto mode: Submit directly to AI with screenshot
          const attachedFile: AttachedFile = {
            id: Date.now().toString(),
            name: `screenshot_${Date.now()}.png`,
            type: "image/png",
            base64: base64,
            size: base64.length,
          };

          // Generate unique request ID
          const requestId = generateRequestId();
          currentRequestIdRef.current = requestId;

          // Cancel any existing request
          if (abortControllerRef.current) {
            abortControllerRef.current.abort();
          }

          abortControllerRef.current = new AbortController();
          const signal = abortControllerRef.current.signal;

          try {
            const useScribeAPI = await shouldUseScribeAPI(selectedAIProvider);
            // Check if AI provider is configured
            if (!selectedAIProvider.provider && !useScribeAPI) {
              setState((prev) => ({
                ...prev,
                error: "Please select an AI provider in settings",
              }));
              return;
            }

            const provider = allAiProviders.find(
              (p) => p.id === selectedAIProvider.provider
            );
            if (!provider && !useScribeAPI) {
              setState((prev) => ({
                ...prev,
                error: "Invalid provider selected",
              }));
              return;
            }

            const trimmedHistory = trimHistoryToContextWindow(
              state.conversationHistory
            );
            const messageHistory = trimmedHistory.map((msg) => ({
              role: msg.role,
              content: msg.content,
            }));

            const conversationIdForFacts =
              state.currentConversationId || "";
            const facts = conversationIdForFacts
              ? await getFactsForConversation(conversationIdForFacts)
              : {};
            const factsEntries = Object.entries(facts);
            const systemPromptWithFacts =
              factsEntries.length > 0
                ? (effectiveSystemPrompt || "") +
                  "\n\nStored facts for this conversation: " +
                  factsEntries.map(([k, v]) => `${k}: ${v}`).join("; ")
                : effectiveSystemPrompt || undefined;

            let fullResponse = "";

            // Clear previous response and set loading state
            setState((prev) => ({
              ...prev,
              input: prompt,
              isLoading: true,
              error: null,
              response: "",
            }));

            // Use the fetchAIResponse function with image and signal
            for await (const chunk of fetchAIResponse({
              provider: useScribeAPI ? undefined : provider,
              selectedProvider: selectedAIProvider,
              systemPrompt: systemPromptWithFacts,
              history: messageHistory,
              userMessage: prompt,
              imagesBase64: [base64],
              signal,
            })) {
              // Only update if this is still the current request
              if (currentRequestIdRef.current !== requestId || signal.aborted) {
                return; // Request was superseded or cancelled
              }

              fullResponse += chunk;
              setState((prev) => {
                const accumulated = prev.response + chunk;
                return {
                  ...prev,
                  response: stripReasoningFromContent(accumulated),
                };
              });
            }

            // Only proceed if this is still the current request
            if (currentRequestIdRef.current !== requestId || signal.aborted) {
              return;
            }

            setState((prev) => ({ ...prev, isLoading: false }));

            // Focus input after screenshot AI response is complete
            setTimeout(() => {
              inputRef.current?.focus();
            }, 100);

            // Save the conversation after successful completion (use stripped response)
            if (fullResponse) {
              const cleanedResponse = stripReasoningFromContent(fullResponse) || fullResponse;
              await saveCurrentConversation(prompt, cleanedResponse, [
                attachedFile,
              ]);
              // Clear input after saving
              setState((prev) => ({
                ...prev,
                input: "",
              }));
            }
          } catch (e: any) {
            // Only show error if this is still the current request and not aborted
            if (currentRequestIdRef.current === requestId && !signal.aborted) {
              console.error("[useCompletion] Error in screenshot AI response:", e);
              const raw = e?.message || e?.toString() || "An error occurred";
              setState((prev) => ({
                ...prev,
                error: normalizeErrorMessage(raw),
                response: "", // Clear response on error
              }));
            }
          } finally {
            // Only update loading state if this is still the current request
            if (currentRequestIdRef.current === requestId && !signal.aborted) {
              setState((prev) => ({ ...prev, isLoading: false }));
            }
          }
        } else {
          // Manual mode: Add to attached files
          const attachedFile: AttachedFile = {
            id: Date.now().toString(),
            name: `screenshot_${Date.now()}.png`,
            type: "image/png",
            base64: base64,
            size: base64.length,
          };

          setState((prev) => ({
            ...prev,
            attachedFiles: [...prev.attachedFiles, attachedFile],
          }));
        }
      } catch (error) {
        console.error("Failed to process screenshot:", error);
        setState((prev) => ({
          ...prev,
          error:
            error instanceof Error
              ? error.message
              : "An error occurred processing screenshot",
          isLoading: false,
        }));
      }
    },
    [
      state.attachedFiles.length,
      state.conversationHistory,
      selectedAIProvider,
      allAiProviders,
      effectiveSystemPrompt,
      saveCurrentConversation,
      inputRef,
    ]
  );

  const onRemoveAllFiles = () => {
    clearFiles();
    setIsFilesPopoverOpen(false);
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey && !e.repeat) {
      e.preventDefault();
      if (!state.isLoading && state.input.trim()) {
        submit();
      }
    }
  };

  const handlePaste = useCallback(
    async (e: React.ClipboardEvent) => {
      // Check if clipboard contains images
      const items = e.clipboardData?.items;
      if (!items) return;

      const hasImages = Array.from(items).some((item) =>
        item.type.startsWith("image/")
      );

      // If we have images, prevent default text pasting and process images
      if (hasImages) {
        e.preventDefault();

        const processedFiles: File[] = [];

        Array.from(items).forEach((item) => {
          if (
            item.type.startsWith("image/") &&
            state.attachedFiles.length + processedFiles.length < MAX_FILES
          ) {
            const file = item.getAsFile();
            if (file) {
              processedFiles.push(file);
            }
          }
        });

        // Process all files
        await Promise.all(processedFiles.map((file) => addFile(file)));
      }
    },
    [state.attachedFiles.length, addFile]
  );

  const isPopoverOpen =
    state.isLoading ||
    state.response !== "" ||
    state.error !== null ||
    keepEngaged ||
    responseOpen;

  useEffect(() => {
    resizeWindow(
      isPopoverOpen || micOpen || messageHistoryOpen || isFilesPopoverOpen
    );
  }, [
    isPopoverOpen,
    micOpen,
    messageHistoryOpen,
    resizeWindow,
    isFilesPopoverOpen,
  ]);

  // Auto scroll to bottom when response updates
  useEffect(() => {
    if (!keepEngaged && state.response && scrollAreaRef.current) {
      const scrollElement = scrollAreaRef.current.querySelector(
        "[data-radix-scroll-area-viewport]"
      );
      if (scrollElement) {
        scrollElement.scrollTo({
          top: scrollElement.scrollHeight,
          behavior: "smooth",
        });
      }
    }
  }, [state.response, keepEngaged]);

  // Keyboard arrow key support for scrolling
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isPopoverOpen) return;

      const activeScrollRef = scrollAreaRef.current || scrollAreaRef.current;
      const scrollElement = activeScrollRef?.querySelector(
        "[data-radix-scroll-area-viewport]"
      ) as HTMLElement;

      if (!scrollElement) return;

      const scrollAmount = 100; // pixels to scroll

      if (e.key === "ArrowDown") {
        e.preventDefault();
        scrollElement.scrollBy({ top: scrollAmount, behavior: "smooth" });
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        scrollElement.scrollBy({ top: -scrollAmount, behavior: "smooth" });
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isPopoverOpen, scrollAreaRef]);

  // Keyboard shortcut for toggling keep engaged mode (Cmd+K / Ctrl+K)
  useEffect(() => {
    const handleToggleShortcut = (e: KeyboardEvent) => {
      // Only trigger when popover is open
      if (!isPopoverOpen) return;

      // Check for Cmd+K (Mac) or Ctrl+K (Windows/Linux)
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setKeepEngaged((prev) => !prev);
        // Focus the input after toggle (with delay to ensure DOM is ready)
        setTimeout(() => {
          inputRef.current?.focus();
        }, 100);
      }
    };

    window.addEventListener("keydown", handleToggleShortcut);
    return () => window.removeEventListener("keydown", handleToggleShortcut);
  }, [isPopoverOpen]);

  const captureScreenshot = useCallback(async () => {
    if (!handleScreenshotSubmit) return;

    const config = screenshotConfigRef.current;

    setIsScreenshotLoading(true);

    try {
      // Check screen recording permission on macOS
      const platform = navigator.platform.toLowerCase();
      if (platform.includes("mac") && !hasCheckedPermissionRef.current) {
        const {
          checkScreenRecordingPermission,
          requestScreenRecordingPermission,
        } = await import("tauri-plugin-macos-permissions-api");

        const hasPermission = await checkScreenRecordingPermission();

        if (!hasPermission) {
          // Request permission
          await requestScreenRecordingPermission();

          // Wait a moment and check again
          await new Promise((resolve) => setTimeout(resolve, 2000));

          const hasPermissionNow = await checkScreenRecordingPermission();

          if (!hasPermissionNow) {
            setState((prev) => ({
              ...prev,
              error:
                "Screen Recording permission required. Please enable it by going to System Settings > Privacy & Security > Screen & System Audio Recording. If you don't see Scribe in the list, click the '+' button to add it. If it's already listed, make sure it's enabled. Then restart the app.",
            }));
            setIsScreenshotLoading(false);
            return;
          }
        }
        hasCheckedPermissionRef.current = true;
      }

      if (config.enabled) {
        const base64 = await invoke("capture_to_base64");

        if (config.mode === "auto") {
          // Auto mode: Submit directly to AI with the configured prompt
          await handleScreenshotSubmit(base64 as string, config.autoPrompt);
        } else if (config.mode === "manual") {
          // Manual mode: Add to attached files without prompt
          await handleScreenshotSubmit(base64 as string);
        }
      } else {
        // Selection Mode: Open overlay to select an area
        isProcessingScreenshotRef.current = false;
        await invoke("start_screen_capture");
      }
    } catch (error) {
      setState((prev) => ({
        ...prev,
        error: "Failed to capture screenshot. Please try again.",
      }));
      isProcessingScreenshotRef.current = false;
    } finally {
      if (config.enabled) {
        setIsScreenshotLoading(false);
      }
    }
  }, [handleScreenshotSubmit]);

  useEffect(() => {
    let unlisten: any;

    const setupListener = async () => {
      unlisten = await listen("captured-selection", async (event: any) => {
        if (isProcessingScreenshotRef.current) {
          return;
        }

        isProcessingScreenshotRef.current = true;
        const base64 = event.payload;
        const config = screenshotConfigRef.current;

        try {
          if (config.mode === "auto") {
            // Auto mode: Submit directly to AI with the configured prompt
            await handleScreenshotSubmit(base64 as string, config.autoPrompt);
          } else if (config.mode === "manual") {
            // Manual mode: Add to attached files without prompt
            await handleScreenshotSubmit(base64 as string);
          }
        } catch (error) {
          console.error("Error processing selection:", error);
        } finally {
          setIsScreenshotLoading(false);
          setTimeout(() => {
            isProcessingScreenshotRef.current = false;
          }, 100);
        }
      });
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [handleScreenshotSubmit]);

  useEffect(() => {
    const unlisten = listen("capture-closed", () => {
      setIsScreenshotLoading(false);
      isProcessingScreenshotRef.current = false;
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const toggleRecording = useCallback(() => {
    setEnableVAD(!enableVAD);
    setMicOpen(!micOpen);
  }, [enableVAD, micOpen]);

  // Cleanup abort controller on unmount
  useEffect(() => {
    return () => {
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
        abortControllerRef.current = null;
      }
      currentRequestIdRef.current = null;
    };
  }, []);

  // register callbacks for global shortcuts
  useEffect(() => {
    globalShortcuts.registerAudioCallback(toggleRecording);
    globalShortcuts.registerInputRef(inputRef.current);
    globalShortcuts.registerScreenshotCallback(captureScreenshot);
  }, [
    globalShortcuts.registerAudioCallback,
    globalShortcuts.registerInputRef,
    globalShortcuts.registerScreenshotCallback,
    toggleRecording,
    captureScreenshot,
    inputRef,
  ]);

  return {
    input: state.input,
    setInput,
    response: state.response,
    setResponse,
    isLoading: state.isLoading,
    error: state.error,
    attachedFiles: state.attachedFiles,
    addFile,
    removeFile,
    clearFiles,
    submit,
    cancel,
    reset,
    setState,
    enableVAD,
    setEnableVAD,
    micOpen,
    setMicOpen,
    currentConversationId: state.currentConversationId,
    conversationHistory: state.conversationHistory,
    loadConversation,
    startNewConversation,
    messageHistoryOpen,
    setMessageHistoryOpen,
    screenshotConfiguration,
    setScreenshotConfiguration,
    handleScreenshotSubmit,
    handleFileSelect,
    handleKeyPress,
    handlePaste,
    isPopoverOpen,
    scrollAreaRef,
    resizeWindow,
    isFilesPopoverOpen,
    setIsFilesPopoverOpen,
    onRemoveAllFiles,
    inputRef,
    captureScreenshot,
    isScreenshotLoading,
    keepEngaged,
    setKeepEngaged,
    showCapabilities,
  };
};
