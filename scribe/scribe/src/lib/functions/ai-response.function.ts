import {
  buildDynamicMessages,
  deepVariableReplacer,
  extractVariables,
  getByPath,
  getStreamingContent,
} from "./common.function";
import { Message, TYPE_PROVIDER } from "@/types";
import { AI_PROVIDERS } from "@/config";
import { getCustomAiProviders } from "@/lib/storage/ai-providers";
import { fetch as tauriFetch } from "@tauri-apps/plugin-http";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import curl2Json from "@bany/curl-to-json";
import { shouldUseScribeAPI } from "./scribe.api";
import { CHUNK_POLL_INTERVAL_MS } from "../chat-constants";
import { GHOST_CAPABILITIES_KNOWLEDGE } from "@/config/ghost-capabilities";

// Scribe AI streaming function
async function* fetchScribeAIResponse(params: {
  systemPrompt?: string;
  userMessage: string;
  imagesBase64?: string[];
  history?: Message[];
  signal?: AbortSignal;
}): AsyncIterable<string> {
  try {
    const {
      systemPrompt,
      userMessage,
      imagesBase64 = [],
      history = [],
      signal,
    } = params;

    // Check if already aborted before starting
    if (signal?.aborted) {
      return;
    }

    // Convert history to the expected format
    let historyString: string | undefined;
    if (history.length > 0) {
      // Create a copy before reversing to avoid mutating the original array
      const formattedHistory = [...history].reverse().map((msg) => ({
        role: msg.role,
        content: [{ type: "text", text: msg.content }],
      }));
      historyString = JSON.stringify(formattedHistory);
    }

    // Handle images - can be string or array
    let imageBase64: any = undefined;
    if (imagesBase64.length > 0) {
      imageBase64 = imagesBase64.length === 1 ? imagesBase64[0] : imagesBase64;
    }

    // Set up streaming event listener BEFORE invoking
    let streamComplete = false;
    const streamChunks: string[] = [];

    console.log("[Scribe API] Setting up event listeners...");
    const unlisten = await listen("chat_stream_chunk", (event) => {
      const chunk = String(event.payload ?? "");
      console.log("[Scribe API] ✅ Received chunk event:", {
        chunkLength: chunk.length,
        preview: chunk.substring(0, 80),
        totalChunks: streamChunks.length + 1
      });
      if (chunk) {
        streamChunks.push(chunk);
      } else {
        console.warn("[Scribe API] ⚠️ Received empty chunk");
      }
    });

    const unlistenComplete = await listen("chat_stream_complete", (event) => {
      const fullResponse = String(event.payload ?? "");
      console.log(
        "[Scribe API] Stream complete event received. Total response length:",
        fullResponse.length
      );
      // If we have a full response but no chunks were received, add it as a single chunk
      if (fullResponse && streamChunks.length === 0) {
        console.log("[Scribe API] No chunks received, using full response as single chunk");
        streamChunks.push(fullResponse);
      }
      streamComplete = true;
    });

    let invokeResolved = false;
    let invokeError: unknown = null;
    let invokeResult: string | null = null;

    try {
      // Check if aborted before starting invoke
      if (signal?.aborted) {
        unlisten();
        unlistenComplete();
        return;
      }

      // Start the streaming request (don't await; let events flow)
      console.log("[Scribe API] Starting chat stream request...");
      const invokePromise = invoke<string>("chat_stream", {
        userMessage,
        systemPrompt,
        imageBase64,
        history: historyString,
      })
        .then((result) => {
          invokeResolved = true;
          if (typeof result === "string") {
            invokeResult = result;
          }
        })
        .catch((error) => {
          invokeResolved = true;
          invokeError = error;
        });

      // Yield chunks as they come in while invoke is running
      let lastIndex = 0;
      let timeoutCount = 0;
      let idleTicks = 0;
      const MAX_TIMEOUTS = 300; // 30 seconds max wait (100ms * 300)

      console.log("[Scribe API] Starting to poll for chunks...");
      while (timeoutCount < MAX_TIMEOUTS) {
        if (signal?.aborted) {
          console.log("[Scribe API] Request aborted");
          unlisten();
          unlistenComplete();
          return;
        }

        await new Promise((resolve) =>
          setTimeout(resolve, CHUNK_POLL_INTERVAL_MS)
        );
        timeoutCount++;

        if (signal?.aborted) {
          console.log("[Scribe API] Request aborted after timeout");
          unlisten();
          unlistenComplete();
          return;
        }

        if (streamChunks.length > lastIndex) {
          console.log(`[Scribe API] 📦 Yielding ${streamChunks.length - lastIndex} new chunks (total in buffer: ${streamChunks.length})`);
        }
        for (let i = lastIndex; i < streamChunks.length; i++) {
          console.log(`[Scribe API] ⬆️ Yielding chunk #${i + 1}: ${streamChunks[i].length} chars`);
          yield streamChunks[i];
        }
        if (streamChunks.length > lastIndex) {
          idleTicks = 0;
        } else {
          idleTicks += 1;
        }
        lastIndex = streamChunks.length;

        // Only break if invoke resolved AND (stream complete OR no new chunks for 5 ticks)
        // This ensures we don't exit before all buffered chunks are yielded
        if (invokeResolved && streamComplete && lastIndex === streamChunks.length && idleTicks >= 5) {
          console.log("[Scribe API] Stream complete and all chunks yielded, exiting poll loop");
          break;
        }
        
        // Also break if invoke resolved with error and we've waited enough
        if (invokeResolved && invokeError && idleTicks >= 10) {
          console.log("[Scribe API] Invoke error detected, exiting poll loop");
          break;
        }
      }

      if (timeoutCount >= MAX_TIMEOUTS) {
        console.warn("[Scribe API] Timeout waiting for stream completion");
      }

      if (invokeError && streamChunks.length === 0) {
        throw invokeError;
      }

      if (invokeResolved && streamChunks.length === 0 && invokeResult && invokeResult.length > 0) {
        console.log("[Scribe API] Using invoke result as response");
        yield invokeResult;
      }

      // Final sweep: yield any remaining chunks that arrived after poll loop exited
      if (lastIndex < streamChunks.length) {
        console.log(`[Scribe API] 🧹 Final sweep: yielding ${streamChunks.length - lastIndex} remaining chunks`);
        for (let i = lastIndex; i < streamChunks.length; i++) {
          console.log(`[Scribe API] ⬆️ Final chunk #${i + 1}: ${streamChunks[i].length} chars`);
          yield streamChunks[i];
        }
      } else {
        console.log(`[Scribe API] ✅ All ${streamChunks.length} chunks yielded, no final sweep needed`);
      }

      await invokePromise;
    } finally {
      unlisten();
      unlistenComplete();
    }
  } catch (error) {
    console.error("[Scribe API] Error in fetchScribeAIResponse:", error);
    const errorMessage = error instanceof Error ? error.message : String(error);
    // Don't yield error as content - throw it so the UI can handle it
    throw new Error(`Scribe API Error: ${errorMessage}`);
  }
}

export async function* fetchAIResponse(params: {
  provider: TYPE_PROVIDER | undefined;
  selectedProvider: {
    provider: string;
    variables: Record<string, string>;
  };
  systemPrompt?: string;
  history?: Message[];
  userMessage: string;
  imagesBase64?: string[];
  signal?: AbortSignal;
}): AsyncIterable<string> {
  try {
    const {
      provider,
      selectedProvider,
      systemPrompt,
      history = [],
      userMessage,
      imagesBase64 = [],
      signal,
    } = params;

    // Check if already aborted
    if (signal?.aborted) {
      return;
    }

    const augmentedSystemPrompt = ((systemPrompt || "").trim() + "\n\n" + GHOST_CAPABILITIES_KNOWLEDGE).trim();

    // If Ollama or Exora AI is explicitly selected, bypass Scribe API regardless of settings
    const isOllamaSelected = selectedProvider?.provider === "ollama" || provider?.id === "ollama" || 
                             selectedProvider?.provider === "exora" || provider?.id === "exora";
    console.log("[fetchAIResponse] Checking provider selection:", {
      isOllamaSelected,
      providerId: provider?.id,
      selectedProviderId: selectedProvider?.provider,
      selectedProviderKeys: selectedProvider ? Object.keys(selectedProvider) : [],
      selectedProviderVariables: selectedProvider?.variables ? Object.keys(selectedProvider.variables) : []
    });
    
    // Check if we should use Scribe API (use selectedProvider from caller as source of truth)
    // For Exora/Ollama: use Scribe (OpenRouter) when user selected a free model; otherwise use direct Ollama
    // For other providers: use Scribe only when license+enabled, else use direct
    let useScribeAPI = await shouldUseScribeAPI(selectedProvider);
    if (isOllamaSelected && !useScribeAPI) {
      console.log("[fetchAIResponse] Exora/Ollama with no free model selected, using direct Ollama");
    } else if (isOllamaSelected && useScribeAPI) {
      console.log("[fetchAIResponse] Exora/Ollama with free model selected, using Scribe API (OpenRouter)");
    }
    console.log("[fetchAIResponse] Final decision - useScribeAPI:", useScribeAPI);
    
    if (useScribeAPI) {
      console.log("[fetchAIResponse] Using Scribe API path");
      yield* fetchScribeAIResponse({
        systemPrompt: augmentedSystemPrompt,
        userMessage,
        imagesBase64,
        history,
        signal,
      });
      return;
    }
    
    console.log("[fetchAIResponse] Using direct provider path");
    // Resolve provider when undefined (e.g. useCompletion passed undefined expecting Scribe,
    // but we forced direct mode for exora/ollama)
    let resolvedProvider = provider;
    if (!resolvedProvider && selectedProvider?.provider) {
      const allProviders = [...AI_PROVIDERS, ...getCustomAiProviders()];
      resolvedProvider = allProviders.find(
        (p) => p.id === selectedProvider.provider
      ) ?? undefined;
    }
    if (!resolvedProvider) {
      throw new Error(
        `Provider not provided. Please select an AI provider in settings (e.g. Exora AI, Ollama, or another provider).`
      );
    }
    if (!selectedProvider) {
      throw new Error(`Selected provider not provided`);
    }
    
    console.log("[fetchAIResponse] Provider found:", resolvedProvider.id, "Variables:", Object.keys(selectedProvider.variables || {}));

    let curlJson;
    try {
      curlJson = curl2Json(resolvedProvider.curl);
    } catch (error) {
      throw new Error(
        `Failed to parse curl: ${
          error instanceof Error ? error.message : "Unknown error"
        }`
      );
    }

    const extractedVariables = extractVariables(resolvedProvider.curl);
    const requiredVars = extractedVariables.filter(
      ({ key }) => key !== "SYSTEM_PROMPT" && key !== "TEXT" && key !== "IMAGE"
    );
    console.log("[fetchAIResponse] Extracted variables:", requiredVars.map(v => v.key));
    
    for (const { key } of requiredVars) {
      // For Ollama and Exora AI, API_KEY is optional (local instances don't need it)
      if ((resolvedProvider.id === "ollama" || resolvedProvider.id === "exora") && key === "api_key") {
        // Allow empty API_KEY for Ollama/Exora AI
        console.log("[fetchAIResponse] Skipping API_KEY check for Ollama/Exora AI");
        continue;
      }
      const varValue = selectedProvider.variables?.[key];
      if (!varValue || varValue.trim() === "") {
        console.error(`[fetchAIResponse] Missing required variable: ${key}. Available variables:`, Object.keys(selectedProvider.variables || {}));
        const errorMsg = (resolvedProvider.id === "ollama" || resolvedProvider.id === "exora") && key === "model"
          ? `Missing required variable: ${key}. Please enter your Ollama model name (e.g., "llama2", "mistral", "qwen") in the settings.`
          : `Missing required variable: ${key}. Please configure it in settings.`;
        throw new Error(errorMsg);
      }
      console.log(`[fetchAIResponse] Variable ${key} is set:`, varValue.substring(0, 20));
    }

    if (!userMessage) {
      throw new Error("User message is required");
    }
    if (imagesBase64.length > 0 && !resolvedProvider.curl.includes("{{IMAGE}}")) {
      throw new Error(
        `Provider ${resolvedProvider?.id ?? "unknown"} does not support image input`
      );
    }

    let bodyObj: any = curlJson.data
      ? JSON.parse(JSON.stringify(curlJson.data))
      : {};
    const messagesKey = Object.keys(bodyObj).find((key) =>
      ["messages", "contents", "conversation", "history"].includes(key)
    );

    if (messagesKey && Array.isArray(bodyObj[messagesKey])) {
      const finalMessages = buildDynamicMessages(
        bodyObj[messagesKey],
        history,
        userMessage,
        imagesBase64
      );
      bodyObj[messagesKey] = finalMessages;
    }

    const allVariables = {
      ...Object.fromEntries(
        Object.entries(selectedProvider.variables).map(([key, value]) => [
          key.toUpperCase(),
          value || "", // Ensure empty string if value is missing
        ])
      ),
      SYSTEM_PROMPT: augmentedSystemPrompt,
    };

    // For Ollama and Exora AI, if API_KEY is not provided, use empty string
    if ((resolvedProvider.id === "ollama" || resolvedProvider.id === "exora") && !allVariables["API_KEY"]) {
      allVariables["API_KEY"] = "";
    }

    bodyObj = deepVariableReplacer(bodyObj, allVariables);
    let url = deepVariableReplacer(curlJson.url || "", allVariables);

    const headers = deepVariableReplacer(curlJson.header || {}, allVariables);
    headers["Content-Type"] = "application/json";
    
    // Remove Authorization header if it's empty (for Ollama)
    if (headers["Authorization"] === "Bearer " || headers["authorization"] === "Bearer ") {
      delete headers["Authorization"];
      delete headers["authorization"];
    }
    
    console.log("[fetchAIResponse] Final URL:", url);
    console.log("[fetchAIResponse] Final headers:", Object.keys(headers));
    console.log("[fetchAIResponse] Request body keys:", Object.keys(bodyObj));

    if (resolvedProvider?.streaming) {
      if (typeof bodyObj === "object" && bodyObj !== null) {
        const streamKey = Object.keys(bodyObj).find(
          (k) => k.toLowerCase() === "stream"
        );
        if (streamKey) {
          bodyObj[streamKey] = true;
        } else {
          bodyObj.stream = true;
        }
      }
    }

    // Use tauriFetch to bypass CORS (browser fetch blocks cross-origin to localhost:11434)
    const fetchOpts = {
      method: curlJson.method || "POST",
      headers,
      body: curlJson.method === "GET" ? undefined : JSON.stringify(bodyObj),
      signal,
      connectTimeout: 60000,
    };

    let response;
    try {
      console.log("[fetchAIResponse] Making request to:", url);
      response = await tauriFetch(url, fetchOpts);
      console.log("[fetchAIResponse] Response status:", response.status, response.statusText);
    } catch (fetchError) {
      // Check if aborted
      if (
        signal?.aborted ||
        (fetchError instanceof Error && fetchError.name === "AbortError")
      ) {
        console.log("[fetchAIResponse] Request aborted");
        return; // Silently return on abort
      }
      const errMsg =
        fetchError instanceof Error
          ? fetchError.message
          : typeof fetchError === "string"
            ? fetchError
            : JSON.stringify(fetchError ?? "Unknown error");
      console.error("[fetchAIResponse] Network error:", fetchError);
      yield `Network error during API request: ${errMsg}`;
      return;
    }

    if (!response.ok) {
      let errorText = "";
      try {
        errorText = await response.text();
      } catch {}
      yield `API request failed: ${response.status} ${response.statusText}${
        errorText ? ` - ${errorText}` : ""
      }`;
      return;
    }

    if (!resolvedProvider?.streaming) {
      let json;
      try {
        json = await response.json();
      } catch (parseError) {
        yield `Failed to parse non-streaming response: ${
          parseError instanceof Error ? parseError.message : "Unknown error"
        }`;
        return;
      }
      const content =
        getByPath(json, resolvedProvider?.responseContentPath || "") || "";
      yield content;
      return;
    }

    if (!response.body) {
      yield "Streaming not supported or response body missing";
      return;
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";

    while (true) {
      // Check if aborted
      if (signal?.aborted) {
        reader.cancel();
        return;
      }

      let readResult;
      try {
        readResult = await reader.read();
      } catch (readError) {
        // Check if aborted
        if (
          signal?.aborted ||
          (readError instanceof Error && readError.name === "AbortError")
        ) {
          return; // Silently return on abort
        }
        yield `Error reading stream: ${
          readError instanceof Error ? readError.message : "Unknown error"
        }`;
        return;
      }
      const { done, value } = readResult;
      if (done) break;

      // Check if aborted before processing
      if (signal?.aborted) {
        reader.cancel();
        return;
      }

      buffer += decoder.decode(value, { stream: true });

      const lines = buffer.split("\n");
      buffer = lines.pop() || "";
      for (const line of lines) {
        if (line.startsWith("data:")) {
          const trimmed = line.substring(5).trim();
          if (!trimmed || trimmed === "[DONE]") {
            console.log("[fetchAIResponse] Stream done");
            continue;
          }
          try {
            const parsed = JSON.parse(trimmed);
            const delta = getStreamingContent(
              parsed,
              resolvedProvider?.responseContentPath || ""
            );
            if (delta) {
              console.log("[fetchAIResponse] Yielding delta:", delta.substring(0, 50));
              yield delta;
            } else {
              console.log("[fetchAIResponse] No delta found in chunk:", Object.keys(parsed));
            }
          } catch (e) {
            // Ignore parsing errors for partial JSON chunks
            console.warn("[fetchAIResponse] Failed to parse chunk:", trimmed.substring(0, 100));
          }
        } else if (line.trim()) {
          console.log("[fetchAIResponse] Non-data line:", line.substring(0, 100));
        }
      }
    }
  } catch (error) {
    throw new Error(
      `Error in fetchAIResponse: ${
        error instanceof Error ? error.message : "Unknown error"
      }`
    );
  }
}
