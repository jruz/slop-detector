pub fn flat_option_handling(a) {
  use user <- option.then(get_user(a))
  use profile <- option.then(get_profile(user))
  Some(profile)
}

pub fn single_case_is_ok(a) {
  case get_user(a) {
    Some(user) -> Some(user)
    None -> None
  }
}
