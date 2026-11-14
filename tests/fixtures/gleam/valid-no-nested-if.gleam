pub fn flat_guards(a, b, c) {
  case a, b, c {
    True, True, True -> "all true"
    True, True, False -> "c false"
    True, False, _ -> "b false"
    False, _, _ -> "a false"
  }
}

pub fn single_case_is_ok(a) {
  case a {
    True -> "true"
    False -> "false"
  }
}
