type SplitResult = readonly [string[], string[]];

export function splitAt(l: string[], idx: number): SplitResult {
  return [l.slice(0, idx), l.slice(idx + 1)];
}
