job "app" {
  group "web" {
    task "server" {
      env {
        APP_HOST          = var.APP_HOST
        API_PORT          = var.API_PORT
        POSTGRES_PASSWORD = var.POSTGRES_PASSWORD
      }
    }
  }
}
