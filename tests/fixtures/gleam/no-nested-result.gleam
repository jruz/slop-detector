pub fn nested_result(a) {
  case fetch_user(a) {
    Ok(user) -> case fetch_profile(user) {
      Ok(profile) -> Ok(profile)
      Error(e) -> Error(e)
    }
    Error(e) -> Error(e)
  }
}
