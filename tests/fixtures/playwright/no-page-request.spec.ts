import { test, expect } from "@playwright/test";

test("fetches via page", async ({ page }) => {
  const response = await page.request.get("/api/data");
  expect(response.ok()).toBeTruthy();
});
