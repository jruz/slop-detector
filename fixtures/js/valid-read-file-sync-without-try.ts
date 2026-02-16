import { readFileSync } from "fs";

export function loadConfig(path: string): string | undefined {
  try {
    return readFileSync(path, "utf-8");
  } catch {
    return undefined;
  }
}
