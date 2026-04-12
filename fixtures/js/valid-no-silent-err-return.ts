import { ok, err, Result } from "neverthrow";

function processData(result: Result<string, Error>) {
  if (result.isErr()) {
    console.error("Failed:", result.error);
    return;
  }
  console.log(result.value);
}

function processData2(result: Result<string, Error>) {
  if (result.isErr()) throw result.error;
}
