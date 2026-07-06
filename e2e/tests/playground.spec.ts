import { test, expect } from "@playwright/test";

test.describe("playground demo", () => {
  test("counter increments when clicked", async ({ page }) => {
    await page.goto("/");
    const button = page.locator("#counter-demo button");
    await expect(button).toHaveText("Count: 0");
    await button.click();
    await expect(button).toHaveText("Count: 1");
    await button.click();
    await expect(button).toHaveText("Count: 2");
  });

  test("todo list: toggle, add, and remove items", async ({ page }) => {
    await page.goto("/");
    const list = page.locator("#todo-demo ul");
    await expect(list.locator("li")).toHaveCount(2);

    const firstItem = list.locator("li").first();
    await firstItem.locator("input[type=checkbox]").check();
    await expect(firstItem).toHaveClass(/done/);

    await page.locator("#todo-demo button", { hasText: "Add" }).click();
    await expect(list.locator("li")).toHaveCount(3);

    const secondItemText = await list.locator("li").nth(1).locator("span").textContent();
    await list.locator("li").nth(1).locator("button", { hasText: "Remove" }).click();
    await expect(list.locator("li")).toHaveCount(2);
    const remainingTexts = await list.locator("li span").allTextContents();
    expect(remainingTexts).not.toContain(secondItemText);
  });

  test("toggling an item preserves the list container's DOM node identity (keyed reconciliation, not a full re-render)", async ({
    page,
  }) => {
    await page.goto("/");
    const list = page.locator("#todo-demo ul");
    const before = await list.elementHandle();
    await list.locator("li").first().locator("input[type=checkbox]").check();
    const after = await list.elementHandle();
    const sameNode = await page.evaluate(([a, b]) => a === b, [before, after]);
    expect(sameNode).toBe(true);
  });
});
