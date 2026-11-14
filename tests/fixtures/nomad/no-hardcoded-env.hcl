job "app" {
  group "web" {
    task "server" {
      env {
        APP_HOST          = var.domain
        API_PORT          = "8080"
        FRONTEND_PORT     = "80"
        POSTGRES_HOST     = "localhost"
        POSTGRES_PORT     = "5432"
        POSTGRES_USER     = var.POSTGRES_USER
        POSTGRES_PASSWORD = var.POSTGRES_PASSWORD
        POSTGRES_DB       = var.POSTGRES_DB
      }
    }
  }
}
