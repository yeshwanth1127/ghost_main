import { fetchAIResponse } from "./ai-response.function";
import { Message } from "@/types";

/**
 * Plan an action using LLM with Action Schema v2 format
 */
export async function planWithLLM(
  userInput: string,
  history: Message[] = []
): Promise<any> {
  // Create a specialized prompt for action planning
  const systemPrompt = `You are an action planner. Convert user requests into a structured action plan following Action Schema v2 JSON format.

Action Schema v2 Format:
{
  "id": "uuid",
  "origin": {
    "user_input": "original user request",
    "source": "ui|voice|automation|plugin",
    "request_id": "uuid"
  },
  "actions": [
    {
      "id": "unique-action-id",
      "type": "fs_create_file|fs_read_file|fs_copy_file|fs_move_file|fs_delete_file|fs_create_directory",
      "args": {
        "path": "absolute/path/to/file",
        "content": "file content (for create)",
        "source_path": "absolute/path/to/source (for copy/move)",
        "destination_path": "absolute/path/to/destination (for copy/move)",
        "encoding": "utf-8"
      },
      "preconditions": {
        "exists": true|false,
        "writable": true|false,
        "readable": true|false
      },
      "metadata": {
        "confidence": 0.0-1.0
      }
    }
  ],
  "summary": "Brief description of the plan",
  "risk_score": 0.0-1.0,
  "dry_run": true
}

Important rules:
1. Always use absolute paths (resolve relative paths to user home)
2. Set risk_score: 0.1-0.3 for read/create, 0.5-0.7 for copy/move, 0.7-0.9 for delete
3. Set dry_run: true always
4. Only output valid JSON, no markdown or explanations
5. Generate UUIDs for id and request_id fields`;

  const planningPrompt = `Convert this request into an Action Schema v2 plan:\n\n${userInput}`;

  try {
    // Use existing AI response function
    let fullResponse = "";
    for await (const chunk of fetchAIResponse({
      provider: undefined, // Will use selected provider
      selectedProvider: {
        provider: "", // Will be determined by useCompletion
        variables: {},
      },
      systemPrompt,
      history,
      userMessage: planningPrompt,
      imagesBase64: [],
    })) {
      fullResponse += chunk;
    }

    // Parse JSON response
    // Try to extract JSON from response (handle markdown code blocks)
    let jsonStr = fullResponse.trim();
    
    // Remove markdown code blocks if present
    if (jsonStr.startsWith("```")) {
      const lines = jsonStr.split("\n");
      const startIdx = lines.findIndex((line) => line.includes("```"));
      const endIdx = lines.findIndex((line, idx) => idx > startIdx && line.includes("```"));
      if (endIdx > startIdx) {
        jsonStr = lines.slice(startIdx + 1, endIdx).join("\n");
      }
    }

    // Parse JSON
    const actionPlan = JSON.parse(jsonStr);

    // Validate basic structure
    if (!actionPlan.id || !actionPlan.actions || !Array.isArray(actionPlan.actions)) {
      throw new Error("Invalid action plan structure from LLM");
    }

    return actionPlan;
  } catch (error) {
    throw new Error(
      `Failed to plan action with LLM: ${
        error instanceof Error ? error.message : String(error)
      }`
    );
  }
}

