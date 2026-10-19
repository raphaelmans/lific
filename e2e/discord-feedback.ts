import { strict as assert } from "node:assert";
import { join } from "node:path";
import type { BrowserContext } from "playwright";

export async function checkDiscordFeedback(context: BrowserContext, base: string) {
  const modifier = process.platform === "darwin" ? "Meta" : "Control";
  for (const width of [1440, 390]) {
    const page = await context.newPage();
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(String(error)));
    try {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(`${base}/DEMO/issues`);
      const rows = page.locator("[data-issue-index]");
      await rows.nth(2).waitFor();
      const selected = page.locator('[data-issue-index] [role="checkbox"][aria-checked="true"]');
      const all = page.getByRole("button", { name: /^(Select all|All selected)\s/ });
      const clear = page.getByRole("button", { name: "Clear selection  ·  Esc", exact: true });
      const selectionCount = page.getByText(/^\d+ selected$/, { exact: true });
      async function assertSelection(identifiers: string[], visibleCount: number) {
        await page.waitForFunction((count) =>
          document.querySelectorAll('[data-issue-index] [role="checkbox"][aria-checked="true"]').length === count,
        identifiers.length);
        assert.deepEqual(await selected.evaluateAll((els) =>
          els.map((el) => el.getAttribute("aria-label")).sort()),
        identifiers.map((id) => `Select ${id}`).sort());
        if (identifiers.length) {
          await selectionCount.waitFor();
          assert.equal(await selectionCount.innerText(), `${identifiers.length} selected`);
          assert(await clear.isVisible());
        } else {
          await selectionCount.waitFor({ state: "hidden" });
          await clear.waitFor({ state: "hidden" });
        }
        if (width >= 640) {
          const complete = identifiers.length === visibleCount;
          assert.equal(await all.getAttribute("aria-pressed"), String(complete));
          const count = identifiers.length && !complete ? `${identifiers.length}/${visibleCount}` : visibleCount;
          assert.equal((await all.innerText()).replace(/\s+/g, " ").trim(),
            `${complete ? "All selected" : "Select all"} ${count}`);
        } else {
          assert.equal(await all.count(), 0, "desktop select-all must not be exposed on mobile");
        }
      }
      async function selectVisible() {
        if (width >= 640) await all.click();
        else {
          // Leave search focus without changing the row selection.
          await rows.first().getByRole("checkbox").focus();
          await page.keyboard.press(`${modifier}+a`);
        }
      }
      async function clearSelection() {
        if (width >= 640) await all.click();
        else await clear.click();
      }
      const checkbox = page.getByRole("checkbox", { name: "Select DEMO-1", exact: true });
      await assertSelection([], 3);
      assert.equal(await checkbox.evaluate((el) => getComputedStyle(el).opacity), "1");
      const initialUrl = page.url();
      await rows.first().click({ position: { x: 2, y: 2 } });
      assert.equal(page.url(), initialUrl, "row whitespace must not navigate");
      await page.keyboard.press(`${modifier}+a`);
      await assertSelection(["DEMO-1", "DEMO-2", "DEMO-3"], 3);
      await clearSelection();
      await assertSelection([], 3);
      await checkbox.click();
      await page.getByRole("checkbox", { name: "Select DEMO-2", exact: true }).click();
      await assertSelection(["DEMO-1", "DEMO-2"], 3);
      await selectVisible();
      await assertSelection(["DEMO-1", "DEMO-2", "DEMO-3"], 3);
      await page.getByRole("checkbox", { name: "Select DEMO-3", exact: true }).click();
      await assertSelection(["DEMO-1", "DEMO-2"], 3);

      const downloadEvent = page.waitForEvent("download");
      await page.getByRole("button", { name: "Export", exact: true }).click();
      const download = await downloadEvent;
      assert.equal(download.suggestedFilename(), "DEMO-selected-issues.md");
      const text = await Bun.file((await download.path())!).text();
      assert(text.includes("Smoke issue") && text.includes("Second smoke issue") && text.includes("First smoke comment"));
      assert(!text.includes("Excluded smoke issue"));
      await page.getByRole("button", { name: "Export", exact: true }).waitFor({ state: "visible" });
      await page.route("**/api/export/issues/*", (route) => route.fulfill({ status: 503, body: "unavailable" }));
      let partialDownload = false;
      page.on("download", () => { partialDownload = true; });
      await page.getByRole("button", { name: "Export", exact: true }).click();
      await page.getByRole("alert").filter({ hasText: "Could not export" }).waitFor();
      assert.equal(partialDownload, false);
      await page.unroute("**/api/export/issues/*");
      await clear.click();
      await assertSelection([], 3);

      // Collapsed groups must not join the select-all set.
      const backlog = page.getByRole("button", { name: /^Backlog\s+1$/i });
      await backlog.click();
      await rows.nth(2).waitFor({ state: "detached" });
      await selectVisible();
      await assertSelection(["DEMO-1", "DEMO-2"], 2);
      await clearSelection();
      await assertSelection([], 2);
      await backlog.click();
      await rows.nth(2).waitFor();

      await rows.first().click({ position: { x: 2, y: 2 } });
      await page.keyboard.press("/");
      const search = page.getByPlaceholder("Search issues...");
      await search.fill("Second smoke");
      await page.waitForFunction(() => document.querySelectorAll("[data-issue-index]").length === 1);
      await search.press(`${modifier}+a`);
      assert.equal(await search.evaluate((el: HTMLInputElement) => el.selectionEnd! - el.selectionStart!), "Second smoke".length);
      assert.equal(await selected.count(), 0, "Ctrl+A in search must select text, not rows");
      await assertSelection([], 1);
      await selectVisible();
      await assertSelection(["DEMO-2"], 1);
      await clearSelection();
      await assertSelection([], 1);
      await search.fill("");
      await search.press("Escape");
      await rows.nth(2).waitFor();

      const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
      assert.equal(overflow, false, `list overflow at ${width}px`);
      assert(!(await page.locator("body").innerText()).includes("Lucide:Terminal"), "malformed icon leaked text");
      if (process.env.LIFIC_SCREENSHOT_DIR) await page.screenshot({ path: join(process.env.LIFIC_SCREENSHOT_DIR, `lific-discord-list-${width}.png`) });
      await page.getByRole("button", { name: "Smoke issue", exact: true }).click();
      await page.waitForURL(/DEMO-1$/);
      assert.equal(errors.length, 0, errors.join("\n"));
      console.log(`ok   Discord issue selection, export, icons (${width}px)`);
    } finally {
      await page.close();
    }
  }

  const page = await context.newPage();
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(String(error)));
  try {
    await page.setViewportSize({ width: 390, height: 900 });
    await page.goto(`${base}/settings`);
    await page.getByRole("heading", { name: "Personal workspace", exact: true }).waitFor();
    assert.equal(await page.getByRole("button", { name: "Add custom or named connection" }).count(), 0);
    assert.equal(await page.getByRole("button", { name: /Connect agent|Copy key|Sign out/ }).count(), 0);
    assert.equal(await page.locator('input[type="password"]').count(), 0);
    assert((await page.locator("body").innerText()).includes("lific mcp"));
    const name = page.getByLabel("Display name", { exact: true });
    const originalName = await name.inputValue();
    await name.fill("Personal smoke operator");
    await page.getByRole("button", { name: "Save name", exact: true }).click();
    await page.getByRole("status").filter({ hasText: "Saved." }).waitFor();
    await page.reload();
    await page.waitForFunction(() =>
      (document.querySelector("#local-name") as HTMLInputElement)?.value === "Personal smoke operator");
    await name.fill(originalName);
    await page.getByRole("button", { name: "Save name", exact: true }).click();
    await page.getByRole("status").filter({ hasText: "Saved." }).waitFor();
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
    if (process.env.LIFIC_SCREENSHOT_DIR) await page.screenshot({ path: join(process.env.LIFIC_SCREENSHOT_DIR, "lific-discord-tools-390.png"), fullPage: true });
    assert.equal(errors.length, 0, errors.join("\n"));
    console.log("ok   Personal settings persistence and removed account/remote controls (390px)");
  } finally {
    await page.close();
  }
}
