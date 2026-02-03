pub fn propagate_error(result: Result(String, Error)) -> Result(String, Error) {
  case result {
    Ok(value) -> Ok(value)
    Error(err) -> Error(err)
  }
}

pub fn transform_error(result: Result(String, Error)) -> Result(String, NewError) {
  case result {
    Ok(value) -> Ok(value)
    Error(_) -> Error(NewError)
  }
}

pub fn ok_wildcard_is_fine(result: Result(String, Error)) -> Result(String, Error) {
  case result {
    Ok(_) -> Ok("success")
    Error(err) -> Error(err)
  }
}

pub fn log_then_error(result: Result(String, Error)) -> Result(String, Error) {
  case result {
    Ok(value) -> Ok(value)
    Error(err) -> {
      log(err)
      Error(err)
    }
  }
}
