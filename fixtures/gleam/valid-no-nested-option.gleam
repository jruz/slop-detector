pub fn one_level_nesting_is_ok(a) {
  case get_user(a) {
    Some(user) -> case get_profile(user) {
      Some(profile) -> Some(profile)
      None -> None
    }
    None -> None
  }
}

pub fn flat_option_handling(a) {
  use user <- option.then(get_user(a))
  use profile <- option.then(get_profile(user))
  Some(profile)
}
