pub fn identity_error(result) {
  case result {
    Ok(value) -> do_something(value)
    Error(err) -> Error(err)
  }
}
