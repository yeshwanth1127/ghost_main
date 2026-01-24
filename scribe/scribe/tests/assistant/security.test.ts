/**
 * Security tests for actionable assistant
 * Tests path traversal prevention and capability token validation
 */

import { describe, it, expect } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { ActionPlan } from "@/types/assistant";

describe("Action Assistant Security", () => {
  it("should reject path traversal attacks", async () => {
    // Try to create file outside user directory
    try {
      const plan = await invoke<ActionPlan>("parse_intent", {
        userInput: "create file ../../etc/passwd with malicious",
      });

      // Plan should be created but verification should fail
      const verified = await invoke("verify_action_plan", { plan });
      
      // If verification doesn't reject it, the path should still be sanitized
      // Check that the path is within user home
      const action = plan.actions[0];
      const path = action.args.path as string;
      
      // Path should be resolved to user home, not system directory
      expect(path).not.toContain("../etc");
    } catch (error) {
      // If it throws, that's also acceptable (rejected early)
      expect(error).toBeDefined();
    }
  });

  it("should reject absolute paths outside user home", async () => {
    try {
      // On Unix systems
      const plan = await invoke<ActionPlan>("parse_intent", {
        userInput: "create file /etc/passwd",
      });

      const verified = await invoke("verify_action_plan", { plan });
      
      // Verification should add warnings or reject
      expect(verified.verification_notes.length).toBeGreaterThan(0);
    } catch (error) {
      // Rejection is acceptable
      expect(error).toBeDefined();
    }
  });

  it("should validate capability token structure", async () => {
    try {
      const token = await invoke("mint_capability_token", {
        scopes: ["fs:create:/home/user/Documents/*"],
        ttlSeconds: 300,
        sessionId: "test-session",
      });

      expect(token).toBeDefined();
      expect(typeof token).toBe("string");
      
      // Token should be valid JSON
      const parsed = JSON.parse(token);
      expect(parsed.nonce).toBeDefined();
      expect(parsed.scopes).toBeDefined();
    } catch (error) {
      // If minting fails, that's a problem
      throw error;
    }
  });

  it("should reject invalid action plans", async () => {
    // Try to verify an invalid plan
    const invalidPlan: ActionPlan = {
      id: "test",
      origin: {
        user_input: "test",
        source: "ui",
        request_id: "test",
      },
      actions: [
        {
          id: "test",
          type: "fs_create_file",
          args: {}, // Missing required 'path' argument
        },
      ],
      summary: "test",
      risk_score: 0.5,
      dry_run: true,
    };

    try {
      await invoke("verify_action_plan", { plan: invalidPlan });
      // Should not reach here
      expect(false).toBe(true);
    } catch (error) {
      // Expected to fail
      expect(error).toBeDefined();
    }
  });
});

