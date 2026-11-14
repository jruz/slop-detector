type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };

export function divide(a: number, b: number): Result<number, string> {
  if (b === 0) {
    return { ok: false, error: "Division by zero" };
  }
  return { ok: true, value: a / b };
}

type Options = {
  name: string;
  items: string[];
};

export function process(opts: Options): string {
  return `${opts.name}: ${opts.items.join(", ")}`;
}
