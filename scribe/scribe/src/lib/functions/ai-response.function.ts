import {
  buildDynamicMessages,
  deepVariableReplacer,
  extractVariables,
  getByPath,
  getStreamingContent,
} from "./common.function";
import { Message, TYPE_PROVIDER } from "@/types";
import { fetch as tauriFetch } from "@tauri-apps/plugin-http";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import curl2Json from "@bany/curl-to-json";
import { shouldUseScribeAPI } from "./scribe.api";
import { CHUNK_POLL_INTERVAL_MS } from "../chat-constants";

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
      console.log("[Scribe API] Received chunk:", chunk.substring(0, 50) + "...");
      if (chunk) {
        streamChunks.push(chunk);
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
          console.log(`[Scribe API] Yielding ${streamChunks.length - lastIndex} new chunks`);
        }
        for (let i = lastIndex; i < streamChunks.length; i++) {
          yield streamChunks[i];
        }
        if (streamChunks.length > lastIndex) {
          idleTicks = 0;
        } else {
          idleTicks += 1;
        }
        lastIndex = streamChunks.length;

        if (invokeResolved && streamComplete && idleTicks >= 5) {
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

      for (let i = lastIndex; i < streamChunks.length; i++) {
        yield streamChunks[i];
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

    // Check if we should use Scribe API instead
    const useScribeAPI = await shouldUseScribeAPI();
    if (useScribeAPI) {
      yield* fetchScribeAIResponse({
        systemPrompt,
        userMessage,
        imagesBase64,
        history,
        signal,
      });
      return;
    }
    if (!provider) {
      throw new Error(`Provider not provided`);
    }
    if (!selectedProvider) {
      throw new Error(`Selected provider not provided`);
    }

    let curlJson;
    try {
      curlJson = curl2Json(provider.curl);
    } catch (error) {
      throw new Error(
        `Failed to parse curl: ${
          error instanceof Error ? error.message : "Unknown error"
        }`
      );
    }

    const extractedVariables = extractVariables(provider.curl);
    const requiredVars = extractedVariables.filter(
      ({ key }) => key !== "SYSTEM_PROMPT" && key !== "TEXT" && key !== "IMAGE"
    );
    for (const { key } of requiredVars) {
      if (
        !selectedProvider.variables?.[key] ||
        selectedProvider.variables[key].trim() === ""
      ) {
        throw new Error(
          `Missing required variable: ${key}. Please configure it in settings.`
        );
      }
    }

    if (!userMessage) {
      throw new Error("User message is required");
    }
    if (imagesBase64.length > 0 && !provider.curl.includes("{{IMAGE}}")) {
      throw new Error(
        `Provider ${provider?.id ?? "unknown"} does not support image input`
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
          value,
        ])
      ),
      SYSTEM_PROMPT: systemPrompt || "",
    };

    bodyObj = deepVariableReplacer(bodyObj, allVariables);
    let url = deepVariableReplacer(curlJson.url || "", allVariables);

    const headers = deepVariableReplacer(curlJson.header || {}, allVariables);
    headers["Content-Type"] = "application/json";

    if (provider?.streaming) {
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

    const fetchFunction = url?.includes("http") ? fetch : tauriFetch;

    let response;
    try {
      response = await fetchFunction(url, {
        method: curlJson.method || "POST",
        headers,
        body: curlJson.method === "GET" ? undefined : JSON.stringify(bodyObj),
        signal,
      });
    } catch (fetchError) {
      // Check if aborted
      if (
        signal?.aborted ||
        (fetchError instanceof Error && fetchError.name === "AbortError")
      ) {
        return; // Silently return on abort
      }
      yield `Network error during API request: ${
        fetchError instanceof Error ? fetchError.message : "Unknown error"
      }`;
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

    if (!provider?.streaming) {
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
        getByPath(json, provider?.responseContentPath || "") || "";
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
          if (!trimmed || trimmed === "[DONE]") continue;
          try {
            const parsed = JSON.parse(trimmed);
            const delta = getStreamingContent(
              parsed,
              provider?.responseContentPath || ""
            );
            if (delta) {
              yield delta;
            }
          } catch (e) {
            // Ignore parsing errors for partial JSON chunks
          }
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
