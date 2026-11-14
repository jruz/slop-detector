export function splitAt(l: string[], idx: number) {
  return [l.slice(0, idx), l.slice(idx + 1)] as const;
}
