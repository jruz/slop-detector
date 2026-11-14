import { ok, err, Result } from "neverthrow";

function processData(result: Result<string, Error>) {
  if (result.isErr()) return;
}
