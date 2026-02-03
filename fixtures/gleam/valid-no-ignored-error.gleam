pub fn propagate_error(result) {
  case result {
    Ok(value) -> Ok(value)
    Error(err) -> Error(err)
  }
}

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

pub fn collect_errors(result, errors) {
  case result {
    Ok(value) -> value
    Error(err) -> push(errors, err)
  }
}

pub fn panic_on_error(result) {
  case result {
    Ok(value) -> value
    Error(_) -> panic as "unexpected error"
  }
}
