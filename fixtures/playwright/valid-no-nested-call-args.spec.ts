import { expect, test } from "@playwright/test";

const fetchValue = (): string => "ok";

test("no nested call args", async () => {
    const value = fetchValue();
    expect(value).toBe("ok");
});
