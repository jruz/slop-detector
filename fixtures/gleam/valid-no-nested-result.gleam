pub fn flat_result_handling(a) {
  use user <- result.try(fetch_user(a))
  use profile <- result.try(fetch_profile(user))
  Ok(profile)
}

pub fn single_case_is_ok(a) {
  case fetch_user(a) {
    Ok(user) -> Ok(user)
    Error(e) -> Error(e)
  }
}
