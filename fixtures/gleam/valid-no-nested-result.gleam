pub fn one_level_nesting_is_ok(a) {
  case fetch_user(a) {
    Ok(user) -> case fetch_profile(user) {
      Ok(profile) -> Ok(profile)
      Error(e) -> Error(e)
    }
    Error(e) -> Error(e)
  }
}

pub fn flat_result_handling(a) {
  use user <- result.try(fetch_user(a))
  use profile <- result.try(fetch_profile(user))
  Ok(profile)
}
