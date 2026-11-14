pub fn discard_param() {
  priv_directory("mini_admin")
  |> result.map_error(fn(_) { "error" })
}

pub fn discard_typed_param() {
  priv_directory("mini_admin")
  |> result.map_error(fn(_: Nil) { "error" })
}

pub fn unused_param() {
  priv_directory("mini_admin")
  |> result.map_error(fn(err) { "error" })
}
