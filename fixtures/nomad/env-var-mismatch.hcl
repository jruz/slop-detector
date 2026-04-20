job "app" {
  group "web" {
    task "server" {
      env {
        QDB_PG_USER     = var.QUESTDB_USER
        QDB_PG_PASSWORD = var.QUESTDB_PASSWORD
        API_PORT        = var.API_PORT
      }
    }
  }
}
