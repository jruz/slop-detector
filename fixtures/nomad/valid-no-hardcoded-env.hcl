job "app" {
  group "web" {
    task "server" {
      env {
        APP_HOST          = var.domain
        API_PORT          = var.api_port
        FRONTEND_PORT     = var.frontend_port
        POSTGRES_HOST     = var.postgres_host
        POSTGRES_PORT     = var.postgres_port
        POSTGRES_USER     = var.POSTGRES_USER
        POSTGRES_PASSWORD = var.POSTGRES_PASSWORD
        POSTGRES_DB       = var.POSTGRES_DB
      }
    }
  }
}
