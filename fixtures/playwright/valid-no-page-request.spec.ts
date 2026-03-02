import { test, expect } from "@playwright/test";

test("navigates", async ({ page }) => {
  await page.goto("/");
});
