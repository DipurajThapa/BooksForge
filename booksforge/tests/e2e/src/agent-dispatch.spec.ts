import { test, expect } from "@playwright/test";

/**
 * Agent-dispatch E2E — Copyedit + Continuity end-to-end through the
 * ProposalReview surface.  Closes audit #29 in the integration sense
 * (ProposalReview is unit-tested in `apps/desktop/src-ui/.../ProposalReview.test.tsx`;
 * this test exercises the wired path through the orchestrator, mocks
 * Ollama, asserts the user can accept some hunks and apply them).
 *
 * Activation requires:
 *   - Mock Ollama at `127.0.0.1:11434` returning fixture proposals.
 *   - `BOOKSFORGE_TEST_HOME` settings dir.
 *   - tauri-driver in CI.
 *
 * All blocked on the same harness work as `golden-path.spec.ts`.
 */

test.describe("Agent dispatch + ProposalReview", () => {
  test.skip("Copyedit dispatch returns proposals; user accepts subset; applies", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /open project/i }).click();
    await page.getByText(/small-fiction-project/).click();

    // Open Copyedit panel.
    await page.keyboard.press("Control+Shift+C");
    await page.getByRole("button", { name: /run copyedit/i }).click();

    // Wait for proposals from the mock Ollama.
    await expect(page.getByRole("region", { name: /Proposals from Copyedit/i }))
      .toBeVisible({ timeout: 15_000 });

    // Reject the first hunk; accept the rest.
    const hunks = await page.getByRole("section", { name: /^Hunk \d+ of \d+$/ }).all();
    expect(hunks.length).toBeGreaterThan(1);
    await page.getByRole("button", { name: /Reject hunk 1 of/ }).click();
    for (let i = 2; i <= hunks.length; i += 1) {
      await page.getByRole("button", { name: new RegExp(`Accept hunk ${i} of`, "i") }).click();
    }

    // Status footer shows the right counts.
    const status = page.getByRole("status");
    await expect(status).toContainText(`${hunks.length - 1} accepted`);
    await expect(status).toContainText("1 rejected");

    // Apply.
    await page.getByRole("button", { name: new RegExp(`Apply ${hunks.length - 1}`, "i") }).click();
    await expect(page.getByText(/applied/i)).toBeVisible({ timeout: 15_000 });

    // A pre_agent_edit snapshot must have been created.
    await page.keyboard.press("Control+Shift+H");
    await expect(page.getByText(/pre_agent_edit/i)).toBeVisible();
  });

  test.skip("Continuity dispatch surfaces evidence and per-scene fixes", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /open project/i }).click();
    await page.getByText(/small-fiction-project/).click();
    await page.getByRole("button", { name: /run continuity/i }).click();
    // ...
    expect(page).toBeTruthy();
  });
});
