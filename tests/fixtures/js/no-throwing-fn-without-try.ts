import { readFileSync } from "fs";

export function loadConfig(path: string): string {
  return readFileSync(path, "utf-8");
}
