import { test, expect } from "@playwright/test";

test("fetches data", async ({ request }) => {
  const response = await request.get("/api/data");
  expect(response.ok()).toBeTruthy();
});
