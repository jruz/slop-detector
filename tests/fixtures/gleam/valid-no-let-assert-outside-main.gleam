pub fn main() {
  let assert Ok(value) = get_result()
  value
}

pub fn helper() {
  case get_result() {
    Ok(value) -> value
    Error(_) -> panic as "failed"
  }
}
