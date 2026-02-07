pub fn transform_error(result) {
  case result {
    Ok(value) -> Ok(value)
    Error(err) -> Error(format_error(err))
  }
}

pub fn log_then_error(result) {
  case result {
    Ok(value) -> Ok(value)
    Error(err) -> {
      log(err)
      Error(err)
    }
  }
}

pub fn discard_error(result) {
  case result {
    Ok(value) -> value
    Error(_) -> "fallback"
  }
}

pub fn use_error(result) {
  case result {
    Ok(value) -> value
    Error(err) -> panic as err
  }
}
