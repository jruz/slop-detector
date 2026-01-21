type Options = {
  name?: string;
  items?: string[];
};

export function process(opts: Options): string {
  const name = opts.name ?? "";
  const items = opts.items ?? [];
  return `${name}: ${items.join(", ")}`;
}
