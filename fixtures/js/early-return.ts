export function validate(input: string): void {
  if (!input) {
    return;
  }
  console.log(`Valid: ${input}`);
}
