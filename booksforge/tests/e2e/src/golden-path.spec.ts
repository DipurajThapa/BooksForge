import { test, expect } from "@playwright/test";

/**
 * Golden-path E2E — the seven journeys from `outputs/MVP_SCOPE.md §6`.
 *
 *   1. Intake
 *   2. Outline
 *   3. Drafting
 *   4. Revising
 *   5. Continuity
 *   6. Copyedit
 *   7. Humanization → Export
 *
 * The test walks all seven against a small fiction-project fixture.
 *
 * Activation: switch `test.skip` to `test` once `tauri-driver` is
 * wired into the test harness (see README) AND the app accepts a
 * `BOOKSFORGE_TEST_HOME` env var that points at a temp settings
 * directory.  Both are MZ-09 follow-ups.
 */

test.describe("Golden path — the seven journeys", () => {
  test.skip("walks intake → outline → drafting → revising → continuity → copyedit → export", async ({ page }) => {
    // 1. Open the app — picker view appears.
    await page.goto("/"); // Tauri-driver routes / to the app's root.
    await expect(page.getByRole("heading", { name: /BooksForge/i })).toBeVisible();

    // 2. Create a new project via the wizard.
    await page.getByRole("button", { name: /new project/i }).click();
    await page.getByLabel(/Project title/i).fill("Test Novel");
    await page.getByLabel(/Author/i).fill("Test Author");
    await page.getByRole("button", { name: /create/i }).click();

    // 3. Editor shell loads with an empty Binder.
    await expect(page.getByText(/Drag a chapter here/i)).toBeVisible();

    // 4. Run Intake → Outline (Tier 1 agent).
    await page.getByRole("button", { name: /run intake/i }).click();
    // (in real run: brief is filled in by the test fixture)

    // 5. Outline-Architect produces a proposal — accept and apply.
    await expect(page.getByText(/Outline Architect suggestions/i)).toBeVisible({ timeout: 30_000 });
    await page.getByRole("button", { name: /accept all/i }).click();
    await page.getByRole("button", { name: /apply/i }).click();

    // 6. Binder now has chapters.
    await expect(page.getByText(/Chapter 1/i)).toBeVisible();

    // 7. Edit a scene via the TipTap editor.
    await page.getByText(/Chapter 1/i).click();
    const editor = page.locator(".ProseMirror");
    await editor.click();
    await editor.fill("Test prose for the golden-path walk.");

    // 8. Wait for autosave (5s default) — saved indicator should change.
    await page.waitForTimeout(6_000);
    await expect(page.getByText(/saved/i)).toBeVisible();

    // 9. Take a manual snapshot.
    await page.keyboard.press("Control+Shift+S"); // app.snapshot.create
    await expect(page.getByText(/Snapshot taken/i)).toBeVisible();

    // 10. Run Copyedit, accept, apply.
    await page.getByRole("button", { name: /run copyedit/i }).click();
    await expect(page.getByText(/Copyedit suggestions/i)).toBeVisible({ timeout: 30_000 });
    await page.getByRole("button", { name: /accept all/i }).click();
    await page.getByRole("button", { name: /apply/i }).click();

    // 11. Run Continuity check.
    await page.getByRole("button", { name: /run continuity/i }).click();
    await expect(page.getByText(/Continuity scenes pass/i)).toBeVisible({ timeout: 30_000 });

    // 12. Open Export, pick Markdown profile, export.
    await page.keyboard.press("Control+Shift+E");
    await page.getByRole("button", { name: /Export to MARKDOWN/i }).click();
    await expect(page.getByText(/exported successfully/i)).toBeVisible({ timeout: 30_000 });
  });
});
