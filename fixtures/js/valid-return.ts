export function validate(input: string): string | null {
  if (!input) {
    return null;
  }
  return input.trim();
}
