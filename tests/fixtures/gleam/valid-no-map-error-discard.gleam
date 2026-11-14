pub fn transform_error() {
  priv_directory("mini_admin")
  |> result.map_error(fn(err) { format_error(err) })
}

pub fn use_case_with_nil() {
  case priv_directory("mini_admin") {
    Ok(priv) -> Ok(priv <> "/static")
    Error(Nil) -> Error("Could not find priv directory")
  }
}
