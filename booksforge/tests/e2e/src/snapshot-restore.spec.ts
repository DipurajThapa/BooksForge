import { test, expect } from "@playwright/test";

/**
 * Snapshot + per-node selective restore — exercises audit #31 once
 * the per-node-restore UI lands.  Until then, only the
 * full-project-restore path is covered and the per-node tests are
 * `test.skip`.
 */

test.describe("Snapshot + restore", () => {
  test.skip("create → manual edit → restore returns to snapshot state", async ({ page }) => {
    await page.goto("/");
    // Open existing fixture project.
    await page.getByRole("button", { name: /open project/i }).click();
    // ... open dialog wiring goes here once tauri-driver is in place.
    await page.getByText(/small-fiction-project/).click();

    // Take a snapshot of the current state.
    await page.keyboard.press("Control+Shift+S");
    const snapshotName = await page
      .getByText(/Snapshot \w+/)
      .first()
      .textContent();
    expect(snapshotName).toBeTruthy();

    // Mutate a scene.
    const editor = page.locator(".ProseMirror");
    await editor.fill("This text will be reverted.");
    await page.waitForTimeout(6_000);

    // Restore the snapshot.
    await page.keyboard.press("Control+Shift+H");
    await page.getByText(snapshotName!).click();
    await page.getByRole("button", { name: /restore/i }).click();
    await page.getByRole("button", { name: /confirm/i }).click();

    // Editor should not contain the post-snapshot text.
    await expect(editor).not.toContainText("This text will be reverted.");
  });

  test.skip("per-node selective restore (audit #31)", async ({ page }) => {
    // TODO: implement once the SnapshotsPanel grows the per-node
    // checkbox UI.
    expect(page).toBeTruthy();
  });
});
