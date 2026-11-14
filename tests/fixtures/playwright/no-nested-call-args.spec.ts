import { expect, test } from "@playwright/test";

const fetchValue = (): string => "ok";

test("no nested call args", async () => {
    expect(fetchValue()).toBe("ok");
});
