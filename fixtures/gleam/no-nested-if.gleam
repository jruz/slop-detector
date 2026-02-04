pub fn deeply_nested_if(a, b, c) {
  case a {
    True -> case b {
      True -> case c {
        True -> "all true"
        False -> "c false"
      }
      False -> "b false"
    }
    False -> "a false"
  }
}
