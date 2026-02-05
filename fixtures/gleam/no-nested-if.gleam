pub fn nested_if(a, b) {
  case a {
    True -> case b {
      True -> "both true"
      False -> "b false"
    }
    False -> "a false"
  }
}
