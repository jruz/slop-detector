pub fn deeply_nested_option(a) {
  case get_user(a) {
    Some(user) -> case get_profile(user) {
      Some(profile) -> case get_settings(profile) {
        Some(settings) -> Some(settings)
        None -> None
      }
      None -> None
    }
    None -> None
  }
}
