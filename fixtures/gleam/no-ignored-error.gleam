pub fn process(result: Result(String, Error)) -> String {
  case result {
    Ok(value) -> value
    Error(_) -> "fallback"
  }
}
