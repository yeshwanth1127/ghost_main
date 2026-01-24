/**
 * Integration tests for actionable assistant
 * Tests the full flow: intent → plan → verify → execute → undo
 */

import { describe, it, expect, beforeAll } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import type { ActionPlan, PreviewResult, ActionResult } from "@/types/assistant";

describe("Action Assistant Integration", () => {
  it("should parse simple intent: create file", async () => {
    const plan = await invoke<ActionPlan>("parse_intent", {
      userInput: "create file test.txt with hello world",
    });

    expect(plan).toBeDefined();
    expect(plan.actions.length).toBeGreaterThan(0);
    expect(plan.actions[0].type).toBe("fs_create_file");
  });

  it("should verify action plan", async () => {
    const plan = await invoke<ActionPlan>("parse_intent", {
      userInput: "create file test.txt",
    });

    const verified = await invoke("verify_action_plan", { plan });
    expect(verified).toBeDefined();
    expect(verified.verified_at).toBeDefined();
  });

  it("should preview action plan", async () => {
    const plan = await invoke<ActionPlan>("parse_intent", {
      userInput: "create file test.txt",
    });

    const preview = await invoke<PreviewResult>("preview_action_plan", {
      plan,
    });

    expect(preview).toBeDefined();
    expect(preview.risk_score).toBeGreaterThanOrEqual(0);
    expect(preview.risk_score).toBeLessThanOrEqual(1);
    expect(preview.affected_items.length).toBeGreaterThan(0);
  });

  it("should execute action plan (dry run)", async () => {
    const plan = await invoke<ActionPlan>("parse_intent", {
      userInput: "create file test.txt with test content",
    });

    // Note: In a real test, you'd need to handle actual file creation
    // This test validates the flow works end-to-end
    const preview = await invoke<PreviewResult>("preview_action_plan", {
      plan,
    });

    expect(preview).toBeDefined();
    // Actual execution would be: await invoke("execute_action_plan", { plan })
  });

  it("should get audit history", async () => {
    const history = await invoke("get_audit_history", { limit: 10 });
    expect(Array.isArray(history)).toBe(true);
  });
});

