pub fn swallow_with_discard(result) {
  case result {
    Ok(value) -> value
    Error(_) -> "fallback"
  }
}

pub fn swallow_with_underscore_name(result) {
  case result {
    Ok(value) -> value
    Error(_err) -> "fallback"
  }
}

pub fn swallow_unused_binding(result) {
  case result {
    Ok(value) -> value
    Error(err) -> "fallback"
  }
}
