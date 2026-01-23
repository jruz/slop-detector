pub fn get_value(opt: Option(String)) -> String {
  case opt {
    Some(value) -> value
    None -> ""
  }
}
