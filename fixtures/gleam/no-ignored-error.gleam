pub fn swallow_with_discard(result: Result(String, Error)) -> String {
  case result {
    Ok(value) -> value
    Error(_) -> "fallback"
  }
}

pub fn swallow_with_underscore_var(result: Result(String, Error)) -> String {
  case result {
    Ok(value) -> value
    Error(_err) -> "fallback"
  }
}

pub fn swallow_with_nil(result: Result(String, Nil)) -> String {
  case result {
    Ok(value) -> value
    Error(Nil) -> "fallback"
  }
}

pub fn swallow_with_binding(result: Result(String, Error)) -> String {
  case result {
    Ok(value) -> value
    Error(err) -> "fallback"
  }
}
