pub fn process(result: Result(String, Error)) -> String {
  case result {
    Ok(value) -> value
    Error(err) -> "failed: " <> err.message
  }
}

pub fn process_with_wildcard(result: Result(String, Error)) -> String {
  case result {
    Ok(_) -> "success"
    Error(err) -> err.message
  }
}
