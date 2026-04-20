job "app" {
  group "web" {
    task "server" {
      env {
        APP_HOST  = var.APP_HOST
        API_PORT  = var.API_PORT
        LOG_FORMAT = var.LOG_FORMAT
      }
    }
  }
}
