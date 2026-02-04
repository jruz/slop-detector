pub fn deeply_nested_result(a) {
  case fetch_user(a) {
    Ok(user) -> case fetch_profile(user) {
      Ok(profile) -> case fetch_settings(profile) {
        Ok(settings) -> Ok(settings)
        Error(e) -> Error(e)
      }
      Error(e) -> Error(e)
    }
    Error(e) -> Error(e)
  }
}
