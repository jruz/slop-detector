export function logError(msg: string): void {
  process.stderr.write(msg + "\n");
}
