pub fn register_page(db, token) {
  case get_invitation(db, token) {
    Ok(result) -> result
    Error(QueryTimeout)
    | Error(ConnectionUnavailable)
    | Error(ConstraintViolated(_, _, _))
    | Error(PostgresqlError(_, _, _))
    | Error(UnexpectedArgumentCount(_, _))
    | Error(UnexpectedArgumentType(_, _))
    | Error(UnexpectedResultType(_)) -> internal_server_error()
  }
}
