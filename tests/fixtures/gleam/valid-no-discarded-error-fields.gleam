pub fn handle_error_properly(db, token) {
  case get_invitation(db, token) {
    Ok(result) -> result
    Error(QueryTimeout) -> handle_timeout()
    Error(ConnectionUnavailable) -> handle_connection()
    Error(ConstraintViolated(name, detail, _hint)) -> handle_constraint(name, detail)
    Error(PostgresqlError(code, message, _detail)) -> handle_pg(code, message)
  }
}

pub fn exhaustive_match(db, token) {
  case get_invitation(db, token) {
    Ok(result) -> result
    Error(err) -> handle_error(err)
  }
}
