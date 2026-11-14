pub fn nested_option(a) {
  case get_user(a) {
    Some(user) -> case get_profile(user) {
      Some(profile) -> Some(profile)
      None -> None
    }
    None -> None
  }
}
