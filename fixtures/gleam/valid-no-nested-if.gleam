pub fn two_levels_is_ok(a, b) {
  case a {
    True -> case b {
      True -> "both true"
      False -> "b false"
    }
    False -> "a false"
  }
}

pub fn flat_guards(a, b, c) {
  case a, b, c {
    True, True, True -> "all true"
    True, True, False -> "c false"
    True, False, _ -> "b false"
    False, _, _ -> "a false"
  }
}
